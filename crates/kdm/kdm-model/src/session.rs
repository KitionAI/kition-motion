//! [`MotionSession`]: one project's replica and its [`MotionModel`] view, for the engine's
//! command bridge.
//!
//! It is [`KitSession<MotionModel>`]'s flow (transactions, per-actor undo, the change exchange,
//! the post-merge repair through [`KitModel::validate`], the view reloaded through
//! [`KitModel::apply`]) with two things [`KitSession`] does not do on its own: a diff-bridged
//! `record` (the engine already holds the project it wants written; this writes only what
//! changed since the view) and counting how many of a `record`'s transactions became undo steps
//! inside an open undo group, so the engine's own history knows how many replica undo steps one
//! of its entries is worth.
//!
//! The model never embeds blobs (footage is referenced by path), so a
//! [`kition_kit::session::MemoryBlobs`] that is never written to is enough.

use kition_kit::kdm::{ActorId, Origin, Replica, UndoOutcome, UndoScope, Version};
use kition_kit::session::{Damage, KitError, KitSession, MemoryBlobs};
use kition_motion_project::Project;

use crate::{Ids, MotionModel, Result, create_replica, normalize, read_document, write_changes};

/// One open project as a KDM replica and the engine's view of it. See the module docs.
pub struct MotionSession {
    inner: KitSession<MotionModel>,
    /// An undo group is open, and whether a transaction in it already made it an undo step.
    group: Option<bool>,
}

impl std::fmt::Debug for MotionSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionSession").field("doc", &self.inner.replica().doc_id()).field("actor", &self.inner.replica().actor()).finish_non_exhaustive()
    }
}

/// The first id [`Project::alloc`] hands out on behalf of `actor`: a per-actor slot of the id
/// space. Engine ids are the objects' own data (see the crate docs), so two replicas that each
/// mint one without exchanging first must not mint the same: each mints from its own slot.
///
/// The line of changes has one writer at a time; what mints beside the document is a draft, a
/// replica forked from it (`@kition/kit-editor` opens a draft's engine as it reopens a document,
/// from the document's bytes and its replica, under a fresh actor), whose objects and ids merge
/// in beside the document's (data model 6.6, rules D1 to D9). The slot is the session's, never
/// the file's: a `.kmotion` holds the first id past every id its content holds
/// ([`Project::content_next_id`]), so opening a project, with or without its stored replica,
/// writes nothing of the actor into what it saves.
///
/// `(((actor as u64) % (2^20 − 1)) + 1) << 32`: 2^20 − 1 slots (two actors share one only by a
/// one-in-a-million chance, and even then collide only on an id both mint before exchanging;
/// the repair gives one of them a repair id), each a contiguous 2^32-id range above the ids a
/// file starts with (those stay with the replica that imported it: no other actor mints below
/// 2^32). The largest slot ends at 2^52, below the repair ids (`crate::repair_id`) and the
/// ±2^53 a KDM integer holds.
pub fn slot_base(actor: ActorId) -> u64 {
    ((actor.get() % ((1u64 << 20) - 1)) + 1) << 32
}

/// How many ids one actor's slot ([`slot_base`]) holds.
pub(crate) const SLOT_SIZE: u64 = 1 << 32;

impl MotionSession {
    /// A new replica holding `project`, written in its creating transaction (not an undo step).
    /// The view is `project` itself (its compositions shared, not copied); its id counter is
    /// raised to this replica's own actor's slot if it was not already past it.
    pub fn create(project: &Project) -> Result<Self> {
        let actor = ActorId::random();
        let (replica, ids) = create_replica(project, actor, &Origin::system("create")).map_err(KitError::from)?;
        let mut inner: KitSession<MotionModel> = KitSession::from_replica(replica, Box::new(MemoryBlobs::default()))?;
        let mut project = project.clone();
        project.next_id = project.next_id.max(slot_base(inner.replica().actor()));
        inner.model_mut().adopt(project, ids);
        Ok(Self { inner, group: None })
    }

    /// Opens a snapshot (or a full change export) of another replica as a new replica of a fresh
    /// actor; the view's id counter starts in that actor's slot.
    pub fn open_snapshot(bytes: &[u8]) -> Result<Self> {
        let inner: KitSession<MotionModel> = KitSession::open_snapshot(bytes, Box::new(MemoryBlobs::default()), ActorId::random())?;
        Ok(Self { inner, group: None })
    }

    /// The view: the project as of the last record, undo, redo or import.
    pub fn project(&self) -> &Project {
        &self.inner.model().project
    }

    pub fn model(&self) -> &MotionModel {
        self.inner.model()
    }

    /// Makes `project` the view without writing anything: for a caller whose own copy holds
    /// exactly what the replica holds (compared with [`crate::same_content`]), so the view
    /// shares its compositions and the next [`MotionSession::record`] compares them by pointer.
    pub fn adopt_view(&mut self, project: &Project) {
        let ids = self.inner.model().ids().clone();
        self.inner.model_mut().adopt(project.clone(), ids);
    }

    /// Writes what differs between the view and `after` as one transaction of `origin`, then
    /// makes `after` the view. Answers how many undo steps that made (0 when nothing replicated
    /// changed, when the transaction joined an undo group that is already a step, or for a
    /// `system` origin, which is never an undo step). On failure nothing is written and the view
    /// is unchanged.
    pub fn record(&mut self, origin: &Origin, after: &Project) -> Result<u32> {
        let mut ids = self.inner.model().ids().clone();
        let before = self.inner.model().project.clone();
        let version_before = self.inner.version();
        self.inner.record(origin, |tx| write_changes(tx, &before, after, &mut ids).map(|_| ()).map_err(KitError::from))?;
        self.inner.model_mut().adopt(after.clone(), ids);
        let wrote = self.inner.version() != version_before;
        let undoable = !matches!(origin.kind, kition_kit::kdm::OriginKind::System | kition_kit::kdm::OriginKind::Import);
        let steps = match &mut self.group {
            _ if !wrote || !undoable => 0,
            None => 1,
            Some(counted) if !*counted => {
                *counted = true;
                1
            }
            Some(_) => 0,
        };
        Ok(steps)
    }

    /// Whether the replica holds `project`'s content (as the reader reads both back, id
    /// counters aside). Errs if `project` cannot be written to a replica at all.
    pub fn holds(&self, project: &Project) -> Result<bool> {
        let (scratch, _) = create_replica(project, ActorId::random(), &Origin::system("create")).map_err(KitError::from)?;
        let (theirs, _) = read_document(&scratch).map_err(KitError::from)?;
        let (mine, _) = read_document(self.inner.replica()).map_err(KitError::from)?;
        Ok(normalize(&theirs) == normalize(&mine))
    }

    /// Records `project` over the replica as one transaction of `origin` (a diff from the view:
    /// engine ids are the objects' own, so whatever `project` shares with the replica keeps its
    /// objects), then makes it the view. For a file that must win over a stored replica that
    /// does not hold it.
    pub fn replace(&mut self, origin: &Origin, project: &Project) -> Result<()> {
        self.end_undo_group();
        self.record(origin, project).map(|_| ())
    }

    /// Applies a file difference without replacing the replica or either undo history.
    pub fn reread(&mut self, project: &Project) -> Result<Damage> {
        let mut ids = self.inner.model().ids().clone();
        let damage = self.inner.reread(|before, tx| write_changes(tx, &before.project, project, &mut ids).map(|_| ()).map_err(KitError::from))?;
        self.inner.model_mut().adopt(project.clone(), ids);
        Ok(damage)
    }

    /// Starts an undo group (ending any open one first): the transactions until
    /// [`MotionSession::end_undo_group`] undo as one step.
    pub fn begin_undo_group(&mut self) -> Result<()> {
        self.end_undo_group();
        self.inner.begin_undo_group()?;
        self.group = Some(false);
        Ok(())
    }

    pub fn end_undo_group(&mut self) {
        self.inner.end_undo_group();
        self.group = None;
    }

    /// Whether an undo group is open: a `record` now joins it.
    pub fn group_open(&self) -> bool {
        self.group.is_some()
    }

    /// Undoes this replica's last undo step in `scope` (never imports or repairs, and never the
    /// other scope's steps, Kition platform KitModel 1.1 M12a), repairs the result and reloads the
    /// view. Answers which steps the undo took off the stack ([`UndoOutcome`]: a step with nothing
    /// left to change, its objects gone since, is passed over and the one below it taken back),
    /// and the damage. An empty stack answers an empty outcome.
    pub fn undo_in(&mut self, scope: UndoScope) -> Result<(UndoOutcome, Damage)> {
        self.group = None;
        if !self.inner.can_undo_in(scope) {
            return Ok((UndoOutcome::default(), Damage::None));
        }
        self.inner.undo_in_outcome(scope)
    }

    /// Redoes this replica's last undone step in `scope`, answering as [`MotionSession::undo_in`].
    pub fn redo_in(&mut self, scope: UndoScope) -> Result<(UndoOutcome, Damage)> {
        self.group = None;
        if !self.inner.can_redo_in(scope) {
            return Ok((UndoOutcome::default(), Damage::None));
        }
        self.inner.redo_in_outcome(scope)
    }

    /// Takes a draft's changes (made on a replica forked from this one) in as one transaction of
    /// `origin` (`Replica::merge_changes`, data model 6.6), the kit's repair of the merged state
    /// inside it: one undo step of the person, also when an agent made the draft (M12a), whose
    /// undo gives back the exact state before. Answers the undo scope the step went to (none for
    /// an empty draft or a `system` origin) and the damage. A refused merge, or one whose repair
    /// fails, changes nothing.
    pub fn merge_changes(&mut self, bytes: &[u8], origin: &Origin) -> Result<(Option<UndoScope>, Damage)> {
        self.end_undo_group();
        self.inner.merge_changes(bytes, origin)
    }

    /// The replicated state merging `bytes` (a draft's changes) would give, the kit's repair of
    /// the merged state included as [`MotionSession::merge_changes`] runs it
    /// (`KitSession::merged_state`), so a draft's preview is what accepting it gives. Nothing
    /// changes.
    pub fn merged_state(&self, bytes: &[u8]) -> Result<serde_json::Value> {
        self.inner.merged_state(bytes)
    }

    /// Writes the state `snapshot` holds (a snapshot or content of this document, as a host's
    /// checkpoint keeps it) over the replica as one transaction of `origin`, keyed by object id,
    /// so every object the two share keeps its id, the kit's repair of the restored state inside
    /// it: one undo step of `origin`'s scope unless it is a `system` or `import` one. Nothing
    /// changes when the states are equal, or when the repair takes back all the restore wrote;
    /// another document's snapshot is refused.
    pub fn restore_state(&mut self, snapshot: &[u8], origin: &Origin) -> Result<Damage> {
        self.end_undo_group();
        self.inner.restore_state(snapshot, origin)
    }

    pub fn can_undo_in(&self, scope: UndoScope) -> bool {
        self.inner.can_undo_in(scope)
    }

    pub fn can_redo_in(&self, scope: UndoScope) -> bool {
        self.inner.can_redo_in(scope)
    }

    /// Imports stored changes that follow this replica's version, in order (ending any open undo
    /// group first, so it stays one step), repairs the result and reloads the view.
    pub fn import_changes(&mut self, bytes: &[u8]) -> Result<Damage> {
        self.end_undo_group();
        self.inner.import_changes(bytes)
    }

    /// Whether the view can be (and so, after any import, undo or redo, was) loaded from the
    /// replica as it is now.
    pub fn view_loaded(&self) -> bool {
        read_document(self.inner.replica()).is_ok()
    }

    pub fn export_changes(&self, since: &Version) -> Result<Vec<u8>> {
        self.inner.export_changes(since)
    }

    pub fn version(&self) -> Version {
        self.inner.version()
    }

    pub fn snapshot(&self) -> Result<Vec<u8>> {
        self.inner.snapshot()
    }

    pub fn state_hash(&self) -> Result<String> {
        self.inner.state_hash()
    }

    pub fn replica(&self) -> &Replica {
        self.inner.replica()
    }

    /// The ids of the view's objects.
    pub fn ids(&self) -> &Ids {
        self.inner.model().ids()
    }
}
