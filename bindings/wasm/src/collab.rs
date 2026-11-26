//! Kition's replica members (engine contract 2): the project as a KDM replica.
//!
//! The engine keeps its own `Session` (the command bridge to the replica,
//! `kition_motion_engine::kdm_bridge`), not a `kition_kit::session::KitSession` directly, so
//! `kition_kit::export_kit_session!` does not apply here; these members are added by hand with
//! the exact JS names and shapes `kition-sdk`'s `session::web` gives, so Kition's host code is
//! the same for every kit. The wasm bindings live in [`crate::web`].
//!
//! Motion never embeds blobs in its replica (footage is referenced by path):
//! [`MotionDocument::blobs`] and [`MotionDocument::add_blobs`] answer and take an empty store,
//! so a host's generic replica code (which always calls both) works unchanged.

use std::sync::Arc;

use kition_kit::KitDocument;
use kition_motion_engine::{Damage, Origin, Services, Session, Version};
use kition_motion_media::MediaPool;
use serde_json::Value;

use crate::{KitServices, MotionDocument};

impl MotionDocument {
    /// Opens a file with the replica a host stored for it (the Kit UI protocol's `replica`
    /// capability: a `.kmotion` cannot carry its own replica): the project is the
    /// replica's, so it keeps its KDM identity across saves. When the replica does not hold what
    /// the file holds (the file was saved past it), the file's content is recorded onto it. A
    /// replica that does not open (damaged, a newer schema) is ignored and the file opens as
    /// [`MotionDocument::open`] opens it.
    ///
    /// Empty `bytes` mean there is no file yet, only the replica (a host opening a shared project
    /// for the first time from its sync room's snapshot, to save it as a new file): the project
    /// is the replica's alone.
    pub fn open_with_replica(name: &str, bytes: &[u8], snapshot: &[u8], _blobs: &[u8]) -> Result<Self, String> {
        let services = Arc::new(KitServices::default());
        let pool = Arc::new(MediaPool::new());
        let restored = (|| -> Result<Session, String> {
            let mut session = Self::new_session(&services, &pool);
            if bytes.is_empty() {
                session.open_snapshot(snapshot).map_err(|error| error.to_string())?;
            } else {
                services.write_file(name, bytes).map_err(|error| error.to_string())?;
                session.open_with_replica(name, snapshot).map_err(|error| error.to_string())?;
            }
            // The project is saved to `name`.
            session.path = Some(name.to_owned());
            Ok(session)
        })();
        match restored {
            Ok(session) => Ok(Self::from_session(session, name, services, pool)),
            Err(_) => Self::open(name, bytes),
        }
    }

    /// Whether the project has made its replica yet: it makes one when first asked for it
    /// ([`MotionDocument::version`] and the rest).
    pub fn has_replica(&self) -> bool {
        self.session.kdm().is_some()
    }

    /// The replica's own document id (stable across saves and across opening another replica of
    /// the same document, never the file's name): Kit UI's generic runtime reads this for the Kit
    /// UI protocol's `replica/info` message. `None` before the project has made a replica.
    pub fn doc_id(&self) -> Option<String> {
        self.session.kdm().map(|kdm| kdm.replica().doc_id().to_string())
    }

    /// The project's replica version (made now if there is none yet).
    pub fn version(&mut self) -> Result<Vec<u8>, String> {
        Ok(self.session.version().map_err(|error| error.to_string())?.encode())
    }

    /// The project's changes since `since` (empty bytes: every change).
    pub fn export_changes(&mut self, since: &[u8]) -> Result<Vec<u8>, String> {
        let since = if since.is_empty() { Version::empty() } else { Version::decode(since).map_err(|error| error.to_string())? };
        self.session.export_changes(&since).map_err(|error| error.to_string())
    }

    /// Imports stored changes that follow the replica's version, in order. Answers what went
    /// stale on screen: always a full repaint (the engine tracks no finer damage on its replica).
    pub fn import_changes(&mut self, bytes: &[u8]) -> Result<Damage, String> {
        let damage = self.session.import_changes(bytes).map_err(|error| error.to_string())?;
        self.lend_footage();
        Ok(if damage == Damage::None { Damage::None } else { Damage::Full })
    }

    /// The project's replica as a snapshot, for a host's store or a draft to open as a new
    /// replica of the same project (opening the saved bytes does not: `.kmotion` carries no
    /// replica).
    pub fn snapshot(&mut self) -> Result<Vec<u8>, String> {
        self.session.snapshot().map_err(|error| error.to_string())
    }

    /// Undoes the project's last local step on its replica. `done` is `false`, not an error, when
    /// there is nothing to undo.
    pub fn undo(&mut self) -> Result<(bool, Damage), String> {
        let done = self.session.undo();
        if done {
            self.lend_footage();
        }
        Ok((done, if done { Damage::Full } else { Damage::None }))
    }

    pub fn redo(&mut self) -> Result<(bool, Damage), String> {
        let done = self.session.redo();
        if done {
            self.lend_footage();
        }
        Ok((done, if done { Damage::Full } else { Damage::None }))
    }

    pub fn can_undo(&self) -> bool {
        !self.session.history.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.session.history.redo.is_empty()
    }

    /// Runs a command as `origin` (a person, an agent): the replica records it as theirs, and it
    /// undoes as that replica's own step. Contract 1's `execute` is this with the local person.
    pub fn execute_as(&mut self, command: &str, params: Value, origin: &Origin) -> Result<Value, String> {
        self.refuse_unknown(command, &params)?;
        if command == crate::render::RENDER {
            return crate::render::render(&self.session, &params);
        }
        crate::rest_document::check(&self.session, command, &params)?;
        crate::rest_delete::check(&self.session, command, &params)?;
        crate::rest_inputs::check(&self.session, command, &params)?;
        crate::rest_geometry::check(&self.session, command, &params)?;
        let layer = params.get("layer").cloned();
        // An agent's command given the layers, comp or time it acts on leaves the active comp, the
        // current times and the selections as they were, whatever it moved on the way, also when
        // it fails; only a command whose work is moving them keeps its effect, and so does one that
        // made a layer or a comp (it selects or opens what it made, so the next step of a run acts
        // on it). A person's commands keep the editor's own behaviour.
        let kept = (origin.kind != kition_kit::kdm::OriginKind::Person && crate::acts::names_what_or_when(&params))
            .then(|| (self.session.call_state(), made(&self.session.project)));
        // Someone else's command that changes nothing (an agent reading the project while the
        // person types or drags) does not end the person's coalescing gesture: the engine forgets
        // the merge key whenever the undo scope switches, so it comes back when nothing changed.
        let (merge, revision) = (self.session.history.merge_key.clone(), self.session.revision);
        let result = self.session.execute_as(command, params, origin).map_err(crate::command_failure);
        if merge.is_some() && self.session.revision == revision && self.session.history.merge_key.is_none() {
            self.session.history.merge_key = merge;
        }
        if let Some((state, before)) = kept
            && (result.is_err() || (!crate::acts::moves_the_editor(command) && !grew(before, made(&self.session.project))))
        {
            self.session.restore_call_state(&state);
        }
        let result = result?;
        crate::rest_document::after(&mut self.session, command);
        self.lend_footage();
        Ok(self.with_design(command, layer, result))
    }

    /// Every blob the project's replica references: always empty, Motion never embeds one.
    pub fn blobs(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }

    /// Adds a draft's or a snapshot's blobs: a no-op, Motion never embeds one.
    pub fn add_blobs(&mut self, _bytes: &[u8]) -> Result<(), String> {
        Ok(())
    }

    /// How many times the project left its replica because it refused a write or an undo. The
    /// project keeps working, undoing from its own history; the next request makes a fresh
    /// replica, a new KDM document whose full [`MotionDocument::snapshot`] a host stores again.
    /// Hosts watch this.
    pub fn kdm_resets(&self) -> u32 {
        u32::try_from(self.session.kdm_resets()).unwrap_or(u32::MAX)
    }
}

/// How many comps and layers `project` has.
fn made(project: &kition_motion_project::Project) -> (usize, usize) {
    project.comps().fold((0, 0), |(comps, layers), (_, comp)| (comps + 1, layers + comp.layers.len()))
}

/// Whether a command made a comp or a layer: there are more than `before` now.
fn grew(before: (usize, usize), now: (usize, usize)) -> bool {
    now.0 > before.0 || now.1 > before.1
}

/// The object model (`KitObjectModel 1.0`) over the document's change exchange: `kition-motion` serves
/// `model.describe` and `model.batch` headless through `kition_kit::session::model::Exchange`.
impl kition_kit::session::model::ReplicaPort for MotionDocument {
    fn model_schema(&self) -> std::sync::Arc<kition_kit::kdm::Schema> {
        <kdm_model::MotionModel as kition_kit::session::KitModel>::schema()
    }

    fn model_export(&mut self, since: &[u8]) -> Result<Vec<u8>, String> {
        self.export_changes(since)
    }

    fn model_import(&mut self, bytes: &[u8]) -> Result<(), String> {
        // Import into scratch first: a refusal must not leave rejected fields in the replica.
        use kition_kit::kdm::{ActorId, Replica};
        let snapshot = self.snapshot()?;
        let before = Replica::load(self.model_schema(), &snapshot, ActorId::random()).map_err(|e| e.to_string())?;
        let mut after = Replica::load(self.model_schema(), &snapshot, ActorId::random()).map_err(|e| e.to_string())?;
        after.import_changes(bytes).map_err(|e| e.to_string())?;
        kdm_model::check_model_edits(&before, &after).map_err(|e| e.to_string())?;
        self.import_changes(bytes).map(|_| ())
    }

    fn model_command(&mut self, command: &str, params: Value, origin: &Origin) -> Result<Value, String> {
        // The agent's export (KHP A41) comes here since the headless kit runs agent calls as the
        // agent: it answers the movie, as `execute` does, and changes nothing.
        if command == crate::EXPORT_MOVIE {
            return self.export_movie(&params);
        }
        // An agent's call is checked against the command's schema first (KHP A18); the kit's own
        // editor calls `execute` with what it built.
        crate::params::check(command, &params).map_err(crate::params_failure)?;
        self.execute_as(command, params, origin)
    }

    fn model_commands(&self) -> Vec<kition_kit::KitCommand> {
        self.commands()
    }

    /// A shared project with no file yet (a server's headless kit opening a sync room): the
    /// replica alone, failing rather than falling back to an empty project. Motion keeps no blobs.
    fn model_open_replica(name: &str, snapshot: &[u8], _blobs: &[u8]) -> Result<Self, String> {
        let services = Arc::new(KitServices::default());
        let pool = Arc::new(MediaPool::new());
        let mut session = Self::new_session(&services, &pool);
        session.open_snapshot(snapshot).map_err(|error| error.to_string())?;
        session.path = Some(name.to_owned());
        Ok(Self::from_session(session, name, services, pool))
    }
}

#[cfg(test)]
mod tests {
    use kition_kit::KitDocument;
    use serde_json::json;

    use super::*;

    /// A 64 × 36 project with a red solid whose opacity is animated: no footage needed.
    fn fixture() -> Vec<u8> {
        let mut doc = MotionDocument::create("Motion.kmotion", 64, 36).unwrap();
        let layer = doc.execute("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" })).unwrap()["layer"].clone();
        doc.execute("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 0, "value": 0 })).unwrap();
        doc.execute("prop.addKey", json!({ "layer": layer, "path": "transform/opacity", "time": 1, "value": 100 })).unwrap();
        doc.save(None).unwrap()
    }

    fn extra_edit(doc: &mut MotionDocument) {
        doc.execute("layer.newSolid", json!({ "name": "Blue", "color": "#0000ff" })).unwrap();
    }

    fn layer_names(doc: &MotionDocument) -> Vec<String> {
        doc.session.active_comp().map(|c| c.layers.iter().map(|l| l.name.clone()).collect()).unwrap_or_default()
    }

    #[test]
    fn version_and_snapshot_make_a_replica_lazily() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert!(!doc.has_replica(), "opening a .kmotion plainly makes no replica");
        let version = doc.version().unwrap();
        assert!(doc.has_replica());
        assert_eq!(doc.version().unwrap(), version, "asking twice makes no further replica");
        assert!(!doc.snapshot().unwrap().is_empty());
    }

    #[test]
    fn undo_redo_report_done_and_damage() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.version().unwrap();
        assert!(!doc.can_undo());
        extra_edit(&mut doc);
        assert!(doc.can_undo());
        let (done, damage) = doc.undo().unwrap();
        assert!(done);
        assert_ne!(damage, Damage::None);
        assert!(!doc.can_undo());
        assert!(doc.can_redo());
        assert!(doc.redo().unwrap().0);
        doc.undo().unwrap();
        let (done, damage) = doc.undo().unwrap();
        assert!(!done, "nothing to undo is a normal false");
        assert_eq!(damage, Damage::None);
    }

    /// An agent's read between two keystrokes of the person's typing leaves them one undo step.
    #[test]
    fn an_agents_read_does_not_split_the_persons_gesture() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let person = Origin::person("user:me");
        let layer = doc.execute("comp.info", json!({})).unwrap()["layers"][0]["id"].clone();
        let opacity =
            |doc: &mut MotionDocument| doc.execute_as("prop.get", json!({ "layer": layer, "path": "transform/opacity" }), &person).unwrap()["value"].clone();
        let before = opacity(&mut doc);
        doc.execute_as("prop.set", json!({ "layer": layer, "path": "transform/opacity", "value": 10, "merge": "g" }), &person).unwrap();
        doc.execute_as("comp.info", json!({}), &Origin::agent("reader")).unwrap();
        doc.execute_as("prop.set", json!({ "layer": layer, "path": "transform/opacity", "value": 20, "merge": "g" }), &person).unwrap();
        assert!(doc.undo().unwrap().0);
        assert_eq!(opacity(&mut doc), before, "one undo takes back the whole gesture");
    }

    #[test]
    fn execute_as_records_the_given_origin() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.execute_as("layer.newNull", json!({}), &Origin::person("user:agent-1")).unwrap();
        assert!(doc.can_undo());
    }

    /// `docId` is `None` before any replica exists, and the replica's own id once one does, the
    /// same after a save and a reopen with that replica.
    #[test]
    fn doc_id_is_none_until_a_replica_exists_then_the_replicas_own_id() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert_eq!(doc.doc_id(), None);
        doc.version().unwrap();
        let id = doc.doc_id().unwrap_or_else(|| panic!("a replica exists"));
        assert!(!id.is_empty());
        let bytes = doc.save(None).unwrap();
        let snapshot = doc.snapshot().unwrap();
        let reopened = MotionDocument::open_with_replica("Motion.kmotion", &bytes, &snapshot, &[]).unwrap();
        assert_eq!(reopened.doc_id(), Some(id), "the reopened replica keeps its document id");
    }

    /// With no file yet (empty bytes), the replica alone opens: another replica's full change export
    /// becomes the same project, identity and version, and it saves as a new `.kmotion`.
    #[test]
    fn a_replica_opens_without_a_file() {
        let mut a = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        a.version().unwrap();
        extra_edit(&mut a);
        let changes = a.export_changes(&[]).unwrap();
        let mut b = MotionDocument::open_with_replica("Shared.kmotion", &[], &changes, &[]).unwrap();
        assert_eq!(b.doc_id(), a.doc_id());
        assert_eq!(b.version().unwrap(), a.version().unwrap());
        assert_eq!(b.session.state_hash().unwrap(), a.session.state_hash().unwrap());
        assert!(!b.dirty());
        assert!(b.session.active_comp().is_some(), "the project's composition is open");
        assert_eq!(layer_names(&b), layer_names(&a));
        assert!(kition_motion_project::Project::from_json(std::str::from_utf8(&b.save(None).unwrap()).unwrap()).is_ok());
        assert!(MotionDocument::open_with_replica("Shared.kmotion", &[], b"not a replica", &[]).is_err());
    }

    #[test]
    fn blobs_are_an_empty_store() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert_eq!(doc.blobs().unwrap(), Vec::<u8>::new());
        doc.add_blobs(&[1, 2, 3]).unwrap();
    }

    /// A project saved with its replica reopens as that replica: same version, same state, the
    /// same frame, not dirty, and it takes new edits (which undo) at once.
    #[test]
    fn a_project_reopens_with_its_stored_replica() {
        let mut a = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        a.version().unwrap();
        extra_edit(&mut a);
        let bytes = a.save(None).unwrap();
        let (snapshot, blobs) = (a.snapshot().unwrap(), a.blobs().unwrap());
        let version = a.version().unwrap();

        let mut b = MotionDocument::open_with_replica("Motion.kmotion", &bytes, &snapshot, &blobs).unwrap();
        assert!(b.has_replica());
        assert_eq!(b.version().unwrap(), version);
        assert_eq!(b.session.state_hash().unwrap(), a.session.state_hash().unwrap());
        assert_eq!(b.render(0.0, 0.0, 1.0, 64, 36).unwrap(), a.render(0.0, 0.0, 1.0, 64, 36).unwrap());
        assert!(!b.dirty());
        assert!(!b.can_undo());
        extra_edit(&mut b);
        assert!(b.dirty());
        assert_ne!(b.version().unwrap(), version);
        assert!(b.undo().unwrap().0);
    }

    /// A damaged (or missing) stored replica never stops the file from opening: it opens
    /// plainly, never as a blank project.
    #[test]
    fn a_damaged_stored_replica_opens_the_file_plainly() {
        let bytes = fixture();
        let a = MotionDocument::open("Motion.kmotion", &bytes).unwrap();
        for snapshot in [&b"not a snapshot"[..], &[][..]] {
            let doc = MotionDocument::open_with_replica("Motion.kmotion", &bytes, snapshot, &[]).unwrap();
            assert_eq!((doc.width(), doc.height()), (a.width(), a.height()));
            assert!(!doc.has_replica(), "the file opened plainly");
        }
        assert!(MotionDocument::open_with_replica("Motion.kmotion", b"not a project", b"not a snapshot", &[]).is_err());
    }

    /// `a` saves with its replica; `b` opens with it. `a` edits further (a keyframe moved and a
    /// layer moved in the stack); `b.importChanges(a.exportChanges(b.version()))` takes them in
    /// order, and `b` holds what `a` holds.
    #[test]
    fn a_document_follows_another_through_export_and_import_changes() {
        let mut a = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        a.version().unwrap();
        extra_edit(&mut a);
        let bytes = a.save(None).unwrap();
        let snapshot = a.snapshot().unwrap();
        let mut b = MotionDocument::open_with_replica("Motion.kmotion", &bytes, &snapshot, &[]).unwrap();
        assert_eq!(b.version().unwrap(), a.version().unwrap(), "b starts from a's own replica");

        let red = a.session.active_comp().and_then(|c| c.layers.iter().find(|l| l.name == "Red")).map(|l| l.id.0).unwrap();
        a.execute("prop.set", json!({ "layer": red, "path": "transform/opacity", "value": 40, "time": 1 })).unwrap();
        a.execute("layer.arrange", json!({ "layers": [red], "to": "front" })).unwrap();

        let since = b.version().unwrap();
        let changes = a.export_changes(&since).unwrap();
        b.import_changes(&changes).unwrap();
        assert_eq!(a.session.state_hash().unwrap(), b.session.state_hash().unwrap());
        assert_eq!(a.version().unwrap(), b.version().unwrap());
        assert_eq!(layer_names(&b), layer_names(&a), "the layer move arrived");
        assert_eq!(layer_names(&b).first().map(String::as_str), Some("Red"));
    }
    /// `kition-motion` serves the object model through `ReplicaPort`: a command and a model edit in
    /// batches reach the engine, and a read-only batch refuses writes.
    #[test]
    fn serves_the_object_model_over_its_change_exchange() {
        use kition_kit::session::model::{Exchange, ModelErrorCode};
        use serde_json::{Value, json};

        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let mut exchange = Exchange::new();
        let described = exchange.describe(&mut doc).unwrap();
        assert_eq!(described["capability"], "KitObjectModel 1.0");
        assert!(described["commands"].as_array().unwrap().iter().any(|command| command["id"] == "layer.newNull"));

        let command = json!({ "ops": [{ "op": "command", "id": "layer.newNull", "params": {} }] });
        assert_eq!(exchange.batch(&mut doc, &command, true).unwrap_err().code, ModelErrorCode::ReadOnly);
        let before = doc.version().unwrap();
        exchange.batch(&mut doc, &command, false).unwrap();
        assert_ne!(doc.version().unwrap(), before, "the command ran in the engine");

        let load = |exchange: &mut Exchange, doc: &mut MotionDocument, target: &str, field: &str| -> Value {
            let answer = exchange.batch(doc, &json!({ "ops": [], "load": [{ "target": target, "fields": [field] }] }), false).unwrap();
            answer["loaded"][0]["fields"][field].clone()
        };
        let mut settings = load(&mut exchange, &mut doc, "root", "attrs")["settings"].clone();
        settings["working_gamma"] = json!(2.4);
        let set = json!({ "ops": [{ "op": "map.set", "target": "root", "field": "attrs", "key": "settings", "value": settings }] });
        exchange.batch(&mut doc, &set, false).unwrap();
        assert_eq!(load(&mut Exchange::new(), &mut doc, "root", "attrs")["settings"]["working_gamma"], 2.4);
    }
}
