//! The command bridge to the project's KDM replica.
//!
//! The session's replica ([`kdm_model::MotionSession`]) is the project's shared source of truth;
//! [`Session::project`] is the engine's own view of it. Commands run exactly as before on the
//! in-memory [`Project`] (through [`Session::edit`], which never changes); [`Session::execute`]
//! then writes what the *whole command* changed as one transaction, diffed against
//! [`Session::synced`] (the project as the replica last saw it), so a command that runs others
//! (`engine.batch`, a grouped step) still replicates as one. Each new history step remembers how
//! many replica undo steps it took ([`crate::KdmSteps`]).
//!
//! **Lazy.** The replica is made when a host first asks for it ([`Session::ensure_kdm`]: its
//! version, snapshot or changes), from the project as it is then: a session no host shares
//! (the desktop app, a script's scratch session) pays nothing.
//!
//! **Undo is per person.** [`Session::undo`]/[`Session::redo`] step the replica as many times as
//! the history step is worth, then take what the replica holds as the project — its own snapshot
//! when the two hold the same (so a single-user session's states, and its saved state, stay the
//! very ones it made), else the replica's (folding in a collaborator's edits that arrived since).
//! A step worth no replica steps (taken before the replica existed, after a reset, or on a line
//! a history jump made) is replayed onto the replica as a change of its own when no collaborator's
//! change arrived since (its snapshot is then exactly right); otherwise the project stays what
//! the replica holds, rather than drop those edits.
//!
//! **Not replicated:** [`crate::EditorState`] (selections, open compositions, current times,
//! clipboards: this person's own) and the history's branches. A jump to another state of the
//! History panel ([`Session::goto_history`]) is replicated as one change that is no one's undo
//! step.
//!
//! **Jobs.** A track or mask-track analysis streams frames into the project while it runs; the
//! bridge holds them back and replicates the whole analysis as one change when it ends (no
//! per-frame writes). Other jobs (offloaded analyses, renders, background tasks) apply their
//! results at once, and the poll that applies them replicates them.
//!
//! [`Session::edit`]'s merge key becomes a KDM undo group: the separate `execute` calls of one
//! continuous gesture (a drag) undo on the replica as the one step the engine's history made them.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kdm_model::MotionSession;
use kdm_model::kition_kit::kdm::{Origin, UndoScope, Version};
use kdm_model::kition_kit::session::Damage;
use kition_motion_project::Project;

use crate::history::HistoryMark;
use crate::{EngineError, Event, Result, Session};

/// The origin of the local person's commands (used when nothing else was set with
/// [`Session::execute_as`]/[`Session::set_origin`]).
pub fn local_person() -> Origin {
    Origin::person("person:local")
}

/// The origin of changes that are no one's undo step: a history jump, a step replayed onto the
/// replica.
fn history_origin() -> Origin {
    Origin::system("history")
}

/// A session's link to its replica, managed by this module.
pub struct Bridge {
    /// The replica, made when a host first asks for it ([`Session::ensure_kdm`]); `None` again
    /// after it refused a write ([`Session::kdm_resets`]).
    pub(crate) replica: Option<Mutex<MotionSession>>,
    /// The project as last written to (or read from) the replica; a command's change is what
    /// [`Session::project`] differs from it by.
    pub(crate) synced: Arc<Project>,
    pub(crate) resets: u64,
    /// Who the running command acts for (`None`: [`local_person`]).
    pub(crate) origin: Option<Origin>,
    /// How deep [`Session::execute`] is nested: only the outermost command is replicated.
    pub(crate) depth: u32,
    /// Where the history stood when a change was held back because an analysis was streaming.
    pub(crate) deferred: Option<HistoryMark>,
    /// No collaborator's change arrived since the history line was last made this session's
    /// own (opening, a reset, a history jump): a step the replica holds nothing of is replayed
    /// onto it as a change of its own.
    pub(crate) clean: bool,
}

impl Bridge {
    pub(crate) fn new(project: Arc<Project>) -> Self {
        Self { replica: None, synced: project, resets: 0, origin: None, depth: 0, deferred: None, clean: true }
    }
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new(Arc::new(Project::default()))
    }
}

pub(crate) fn kdm_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::Other(format!("project replica: {error}"))
}

fn no_replica() -> EngineError {
    EngineError::Other("no project replica yet and none could be made".into())
}

impl Session {
    /// The project's replica, if it has one (see [`Session::ensure_kdm`]). Behind a lock only so
    /// that a session can be shared between threads (renders read it while a command runs); the
    /// session itself never holds the lock across a call.
    pub fn kdm(&self) -> Option<MutexGuard<'_, MotionSession>> {
        self.bridge.replica.as_ref().map(|m| m.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// How many times the project left its replica because the replica refused a write or an
    /// undo (a project it cannot hold, or a bug). The project keeps working, undoing from its own
    /// history; the next request for the replica makes a fresh one, which is a new KDM document:
    /// collaborators need its full [`Session::snapshot`] to join again. Hosts watch this.
    pub fn kdm_resets(&self) -> u64 {
        self.bridge.resets
    }

    /// Drops the replica (a later request makes a fresh one): for a command that replaces the
    /// whole project and its history ([`Session::replace_project`]: new, open, demo).
    pub(crate) fn reset_kdm(&mut self) {
        self.bridge.replica = None;
        self.bridge.synced = self.project.clone();
        self.bridge.deferred = None;
        self.bridge.clean = true;
    }

    /// Who the running (or next) command acts for: the origin of [`Session::execute_as`], or the
    /// local person.
    pub fn origin(&self) -> Origin {
        self.bridge.origin.clone().unwrap_or_else(local_person)
    }

    /// Sets who the following commands act for (`None`: the local person); answers the previous
    /// setting, to put back.
    pub fn set_origin(&mut self, origin: Option<Origin>) -> Option<Origin> {
        let scope = scope_of(origin.as_ref().unwrap_or(&local_person()));
        if scope != self.history_scope
            && let Some(mark) = self.bridge.deferred.take()
        {
            // A change held back for the old scope's history is written as its own first.
            self.record_steps(mark);
        }
        let previous = std::mem::replace(&mut self.bridge.origin, origin);
        self.use_scope(scope);
        previous
    }

    /// Makes `history` the one of `scope` (Kition platform KitModel 1.1 M12a: the person's and the
    /// agents' steps undo separately). A coalescing gesture never spans two scopes: its open
    /// replica undo group ends here.
    pub(crate) fn use_scope(&mut self, scope: UndoScope) {
        if self.history_scope != scope {
            self.history.merge_key = None;
            self.open_or_close_kdm_group(false);
            if !self.history.undo.is_empty() {
                // The other scope's steps are like a collaborator's to this one: a step the
                // replica holds nothing of is no longer replayed from its snapshot.
                self.bridge.clean = false;
            }
            std::mem::swap(&mut self.history, &mut self.other_history);
            self.history_scope = scope;
        }
    }

    /// Runs `step` (an undo or a redo) in `scope` (`None`: the caller's, engine rule E14a).
    pub(crate) fn in_scope<T>(&mut self, scope: Option<UndoScope>, step: impl FnOnce(&mut Self) -> T) -> T {
        let own = scope_of(&self.origin());
        self.use_scope(scope.unwrap_or(own));
        let done = step(self);
        self.use_scope(own);
        done
    }

    /// [`Session::execute`] as `origin` (a person, an agent): the replica records the edit as
    /// theirs, and it undoes as that replica's own step.
    pub fn execute_as(&mut self, id: &str, params: serde_json::Value, origin: &Origin) -> Result<serde_json::Value> {
        let previous = self.set_origin(Some(origin.clone()));
        let r = self.execute(id, params);
        self.set_origin(previous);
        r
    }

    /// The replica, made now from the current project if there is none yet; anything not
    /// replicated yet is written first.
    pub fn ensure_kdm(&mut self) -> Option<&mut MotionSession> {
        self.sync_kdm();
        if self.bridge.replica.is_none() {
            match MotionSession::create(&self.project) {
                Ok(kdm) => {
                    self.bridge.replica = Some(Mutex::new(kdm));
                    self.bridge.synced = self.project.clone();
                    self.bridge.deferred = None;
                }
                Err(_) => return None,
            }
        }
        self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner))
    }

    /// The replica's version (its own actor's and every peer's it has merged).
    pub fn version(&mut self) -> Result<Version> {
        Ok(self.ensure_kdm().ok_or_else(no_replica)?.version())
    }

    /// The replica as a snapshot, for a peer to [`Session::open_snapshot`].
    pub fn snapshot(&mut self) -> Result<Vec<u8>> {
        self.ensure_kdm().ok_or_else(no_replica)?.snapshot().map_err(kdm_error)
    }

    /// SHA-256 of the replicated state: equal on replicas that converged.
    pub fn state_hash(&mut self) -> Result<String> {
        self.ensure_kdm().ok_or_else(no_replica)?.state_hash().map_err(kdm_error)
    }

    /// The replica's changes since `since` (`Version::empty()` for all of them).
    pub fn export_changes(&mut self, since: &Version) -> Result<Vec<u8>> {
        self.ensure_kdm().ok_or_else(no_replica)?.export_changes(since).map_err(kdm_error)
    }

    /// Imports a collaborator's (or stored) changes into the replica. The project follows;
    /// anything local not written yet is written first (even a running analysis's frames so far),
    /// so nothing of it is lost. An import that fails after its changes landed (the repair after
    /// the merge refused) still follows the replica when the view loaded from it; when even the
    /// view could not be loaded, the project leaves the replica ([`Session::kdm_resets`]).
    pub fn import_changes(&mut self, bytes: &[u8]) -> Result<Damage> {
        self.ensure_kdm().ok_or_else(no_replica)?;
        if let Some(mark) = self.bridge.deferred.take() {
            self.record_steps(mark);
        }
        self.flush_kdm();
        let kdm = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)).ok_or_else(no_replica)?;
        let before = kdm.version();
        match kdm.import_changes(bytes) {
            Ok(damage) => {
                if kdm.version() != before {
                    self.follow_kdm();
                }
                Ok(damage)
            }
            Err(error) => {
                if kdm.version() != before {
                    if kdm.view_loaded() {
                        self.follow_kdm();
                    } else {
                        self.detach_kdm();
                    }
                }
                Err(kdm_error(error))
            }
        }
    }

    /// Keeps the live identities and undo histories while importing a clean file reread.
    pub fn reconcile_reread(&mut self, project: &Project) -> Result<()> {
        let mut project = project.clone();
        project.next_id = project.next_id.max(self.project.next_id);
        self.ensure_kdm().ok_or_else(no_replica)?.reread(&project).map_err(kdm_error)?;
        self.follow_kdm();
        Ok(())
    }

    /// Opens another replica's snapshot (or a full change export) as this session's project: a
    /// fresh KDM identity, with the project the replica holds and a clean history (undo starts
    /// empty). [`Session::state`] resets to its defaults, with the first composition open.
    pub fn open_snapshot(&mut self, bytes: &[u8]) -> Result<()> {
        let kdm = MotionSession::open_snapshot(bytes).map_err(kdm_error)?;
        let project = Arc::new(kdm.project().clone());
        self.project = project.clone();
        self.bridge.synced = project.clone();
        self.saved_project = project;
        self.bridge.replica = Some(Mutex::new(kdm));
        self.bridge.resets = 0;
        self.bridge.deferred = None;
        self.bridge.clean = true;
        self.bridge.origin = None;
        self.history = crate::History::default();
        self.other_history = crate::History::default();
        self.state = crate::EditorState { snapping: true, view_layout: 1, ..Default::default() };
        let first = self.project.comps().next().map(|(id, _)| *id);
        if let Some(c) = first {
            self.open_comp(c);
        }
        self.sanitize_state();
        self.revision += 1;
        // A freshly opened replica is clean, as a freshly opened file is.
        self.saved_revision = self.revision;
        self.events.push(Event::ProjectChanged { revision: self.revision });
        Ok(())
    }

    /// Opens the project file at `path` (as File ▸ Open reads it) with the replica a host stored
    /// for it (a `.kmotion` cannot carry its own). The host vouches that the two were saved
    /// together, but the file is what the person saved: when the replica does not hold the
    /// file's content, the file wins — its content is recorded onto the replica as one `import`
    /// change (not an undo step), keeping the replica's own identity, so collaborators get it as
    /// ordinary changes. A file the replica cannot hold is an error (the host opens it plainly).
    pub fn open_with_replica(&mut self, path: &str, snapshot: &[u8]) -> Result<()> {
        self.execute("file.open", serde_json::json!({ "path": path }))?;
        let file = self.project.clone();
        let kept_path = self.path.clone();
        self.open_snapshot(snapshot)?;
        self.path = kept_path;
        let kdm = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)).ok_or_else(no_replica)?;
        if !kdm.holds(&file).map_err(kdm_error)? {
            kdm.replace(&Origin::system("import"), &file).map_err(kdm_error)?;
            self.project = file.clone();
            self.bridge.synced = file.clone();
            self.saved_project = file;
            self.sanitize_state();
            let first = self.project.comps().next().map(|(id, _)| *id);
            if self.state.active_comp.is_none()
                && let Some(c) = first
            {
                self.open_comp(c);
            }
            self.revision += 1;
            self.saved_revision = self.revision;
            self.events.push(Event::ProjectChanged { revision: self.revision });
        }
        Ok(())
    }

    /// Whether a track or mask-track analysis is streaming frames into the project.
    fn streaming(&self) -> bool {
        self.track_job.as_ref().is_some_and(|j| !j.is_finished()) || self.mask_job.as_ref().is_some_and(|j| !j.is_finished())
    }

    /// Replicates whatever the project changed outside any command since the last change was
    /// written (a job's result, a host's direct write), unless an analysis is still streaming
    /// (its whole run is replicated when it ends). Commands do this before they run; frontends
    /// that apply jobs outside commands may call it after polling.
    pub fn sync_kdm(&mut self) {
        if self.bridge.depth > 0 || self.streaming() {
            return;
        }
        if let Some(mark) = self.bridge.deferred.take() {
            self.record_steps(mark);
        }
        if self.history.jumped {
            self.record_jump();
        }
        self.flush_kdm();
    }

    /// Writes whatever the project changed since [`Session::synced`] as one transaction, added to
    /// the top history step (a change no command made a step of its own for).
    pub(crate) fn flush_kdm(&mut self) {
        if Arc::ptr_eq(&self.bridge.synced, &self.project) {
            return;
        }
        self.open_or_close_kdm_group(false);
        let origin = self.origin();
        let after = self.project.clone();
        let steps = self.record_kdm(&origin, &after);
        if let Some(top) = self.history.undo.last().map(|e| e.1.clone()) {
            self.history.kdm.add_undo(&top, steps);
        }
    }

    /// After [`Session::execute`] runs one top-level command (or a poll applies a job's result
    /// outside any command): replicates what it changed, one transaction per history step it
    /// pushed (normally exactly one), or added to the top step when it pushed none (a merged
    /// continuation of a gesture, a change that makes no step). `mark` is where the history
    /// stood before.
    pub(crate) fn sync_kdm_after_command(&mut self, mark: HistoryMark) {
        if self.bridge.depth > 0 {
            return;
        }
        if self.streaming() {
            if self.bridge.deferred.is_none() {
                self.bridge.deferred = Some(mark);
            }
            return;
        }
        let mark = self.bridge.deferred.take().unwrap_or(mark);
        if self.history.jumped {
            self.record_jump();
            return;
        }
        self.record_steps(mark);
    }

    fn record_steps(&mut self, mark: HistoryMark) {
        if Arc::ptr_eq(&self.bridge.synced, &self.project) {
            return;
        }
        let origin = self.origin();
        let merging = self.history.merge_key.is_some();
        let pushed = self.history.pushed_since(&mark);
        if pushed.is_empty() {
            let after = self.project.clone();
            let steps = self.record_kdm(&origin, &after);
            if let Some(top) = self.history.undo.last().map(|e| e.1.clone()) {
                self.history.kdm.add_undo(&top, steps);
            }
            return;
        }
        let end = pushed.end;
        for i in pushed {
            let after = self.history.undo.get(i + 1).map(|e| e.1.clone()).unwrap_or_else(|| self.project.clone());
            if Arc::ptr_eq(&self.bridge.synced, &after) {
                continue;
            }
            let is_last = i + 1 == end;
            self.open_or_close_kdm_group(is_last && merging);
            let steps = self.record_kdm(&origin, &after);
            if let Some(state) = self.history.undo.get(i).map(|e| e.1.clone()) {
                self.history.kdm.set_undo(&state, steps);
            }
        }
        // A step whose recorded state is not the project (should not happen) leaves the rest
        // to the next sync, added to the top step.
        if !Arc::ptr_eq(&self.bridge.synced, &self.project) {
            self.flush_kdm();
        }
    }

    /// A history jump: the new state as one change that is no one's undo step.
    fn record_jump(&mut self) {
        self.history.jumped = false;
        self.open_or_close_kdm_group(false);
        if !Arc::ptr_eq(&self.bridge.synced, &self.project) {
            let after = self.project.clone();
            self.record_kdm(&history_origin(), &after);
        }
        self.bridge.clean = true;
    }

    /// Begins a fresh coalescing undo group (ending any open one) when `open_new`, else just ends
    /// whatever is open.
    fn open_or_close_kdm_group(&mut self, open_new: bool) {
        let Some(kdm) = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)) else { return };
        if open_new {
            let _ = kdm.begin_undo_group();
        } else {
            kdm.end_undo_group();
        }
    }

    /// Writes `after` as one transaction on top of the replica, and answers the undo steps that
    /// took. Without a replica there is nothing to write (a host that later asks for one gets it
    /// made from the project as it is by then). A refused write leaves the project exactly as it
    /// is and drops the replica ([`Session::detach_kdm`]).
    fn record_kdm(&mut self, origin: &Origin, after: &Arc<Project>) -> u32 {
        let Some(kdm) = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)) else {
            self.bridge.synced = after.clone();
            return 0;
        };
        match kdm.record(origin, after) {
            Ok(steps) => {
                self.bridge.synced = after.clone();
                steps
            }
            Err(_) => {
                self.detach_kdm();
                0
            }
        }
    }

    /// The replica refused the project: leave it (see [`Session::kdm_resets`]). Every history
    /// step's replica undo-step count goes with it: those steps were the dropped replica's.
    fn detach_kdm(&mut self) {
        self.bridge.replica = None;
        self.bridge.resets = self.bridge.resets.saturating_add(1);
        self.bridge.synced = self.project.clone();
        self.bridge.deferred = None;
        self.bridge.clean = true;
        self.history.kdm = crate::KdmSteps::default();
    }

    /// Undoes (or redoes) `steps` replica transactions and makes the result the project, in
    /// place of the snapshot [`Session::undo`]/[`Session::redo`] already set it to (see the
    /// module docs). `floor` is the project's id counter before the step: the replica's own
    /// reconstruction never lowers it.
    pub(crate) fn step_kdm(&mut self, steps: u32, undo: bool, floor: u64) {
        if self.bridge.replica.is_none() {
            self.bridge.synced = self.project.clone();
            return;
        }
        if steps == 0 {
            if self.bridge.clean {
                self.open_or_close_kdm_group(false);
                let after = self.project.clone();
                self.record_kdm(&history_origin(), &after);
            } else if let Some(kdm) = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)) {
                self.project = view_of(kdm, &self.project, floor);
            }
            self.bridge.synced = self.project.clone();
            return;
        }
        let Some(kdm) = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)) else { return };
        for _ in 0..steps {
            let scope = self.history_scope;
            let ok = matches!(if undo { kdm.undo_in(scope) } else { kdm.redo_in(scope) }, Ok((true, _)));
            if !ok {
                self.detach_kdm();
                return;
            }
        }
        self.project = view_of(kdm, &self.project, floor);
        self.bridge.synced = self.project.clone();
    }

    /// After an import: the project follows the replica, and per-person state that named
    /// something the import removed (a selected layer, the active composition) moves on.
    fn follow_kdm(&mut self) {
        let floor = self.project.next_id;
        let Some(kdm) = self.bridge.replica.as_mut().map(|m| m.get_mut().unwrap_or_else(PoisonError::into_inner)) else { return };
        let mut project = kdm.project().clone();
        project.next_id = project.next_id.max(floor);
        let project = Arc::new(project);
        self.project = project.clone();
        self.bridge.synced = project;
        self.bridge.clean = false;
        self.history.merge_key = None;
        self.sanitize_state();
        self.bump();
    }
}

/// What the replica holds as the session's project: `snapshot` itself (the engine's own state)
/// when it holds just that — its id counter aside, which is the snapshot's own — else the
/// replica's view with an id counter never below `floor` (an id this session handed out may live
/// on in a redo step the reload does not see).
fn view_of(kdm: &mut MotionSession, snapshot: &Arc<Project>, floor: u64) -> Arc<Project> {
    if kdm_model::same_content(kdm.project(), snapshot) {
        kdm.adopt_view(snapshot);
        return snapshot.clone();
    }
    let mut project = kdm.project().clone();
    project.next_id = project.next_id.max(floor);
    Arc::new(project)
}

/// The undo scope `origin`'s steps join (Kition platform KitModel 1.1 M12a): the agents' for an
/// agent, else the person's (imports and repairs are no undo steps and record nothing in a history).
pub(crate) fn scope_of(origin: &Origin) -> UndoScope {
    UndoScope::of(origin.kind).unwrap_or(UndoScope::Person)
}
