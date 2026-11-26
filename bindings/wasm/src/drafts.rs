//! The engine members of contract 2 beyond the change exchange (platform rule K3a): a draft's
//! merge and its comparison, a restored version, the receipt of what changed since a version, an
//! agent's reply in tool ids, and the object model (`model/describe`, `model/batch`), with the
//! names and shapes `kition-sdk`'s `export_kit_session!` gives them (the wasm bindings are in
//! [`crate::web`]). The engine's command bridge takes a merged draft or a restored version in as
//! one undo step, as a command's edit (`Session::merge_draft`, `Session::restore_version`): the
//! project, the person's state and the history follow the replica, and one Undo takes it back.

use std::collections::BTreeSet;

use kition_kit::kdm::diff::diff_states;
use kition_kit::kdm::{ObjectId, Replica, UndoScope};
use kition_kit::session::model::Exchange;
use kition_motion_engine::{Damage, Origin, Version};
use serde_json::{Value, json};

use crate::MotionDocument;

impl MotionDocument {
    /// The replicated state, the replica made first if there is none yet.
    fn replicated_state(&mut self) -> Result<Value, String> {
        self.session.version().map_err(|error| error.to_string())?;
        let kdm = self.session.kdm().ok_or("the project has no replica")?;
        kdm.replica().state_json().map_err(|error| error.to_string())
    }

    /// Takes a draft's changes (from a replica forked from this one) in as one undo step of the
    /// person (`mergeChanges`): the engine's project follows, and Undo takes the whole draft back.
    /// The editor merges an agent's draft with the agent's origin; it is still the person's step
    /// (KitModel M12a). Answers `{"damage", "summary", "scope"}`, `summary` the `StateDiff` of
    /// the replicated state before and after, `scope` the undo history the step went to
    /// (`"person"`; null for an empty draft). A refused merge changes nothing and fails with the KDM's
    /// message.
    pub fn merge_changes(&mut self, bytes: &[u8], origin: &Origin) -> Result<Value, String> {
        let before = self.replicated_state()?;
        let merged = self.session.merge_draft(bytes, origin);
        self.lend_footage();
        let (scope, damage) = merged.map_err(|error| error.to_string())?;
        let after = self.replicated_state()?;
        Ok(json!({ "damage": damage, "summary": diff_states(&before, &after).to_json(), "scope": scope.map(UndoScope::as_str) }))
    }

    /// What merging a draft's changes would change in the project now, as `StateDiff` JSON;
    /// nothing changes. Refuses what [`MotionDocument::merge_changes`] refuses.
    pub fn compare_changes(&self, bytes: &[u8]) -> Result<Value, String> {
        let kdm = self.session.kdm().ok_or("the project has no replica, so no draft of it")?;
        let before = kdm.replica().state_json().map_err(|error| error.to_string())?;
        let after = kdm.replica().merged_state(bytes).map_err(|error| error.to_string())?;
        Ok(diff_states(&before, &after).to_json())
    }

    /// Writes the state a snapshot holds (a checkpoint of this project, the Kit UI protocol's
    /// `replica/restore`) over the project by object id, as one undo step of `origin`'s scope: the
    /// engine's project follows. Nothing changes when the states are equal; another document's
    /// snapshot is refused.
    pub fn restore_state(&mut self, snapshot: &[u8], origin: &Origin) -> Result<Damage, String> {
        let restored = self.session.restore_version(snapshot, origin);
        self.lend_footage();
        restored.map_err(|error| error.to_string())
    }

    /// What changed since the version `since` (empty bytes: since the replica began), a receipt
    /// as `kition-sdk`'s `changesSince` answers it: `{"changes": StateDiff}`, or `{"changes":
    /// null, "unavailable": "<why>"}` when the replica's log does not hold `since` (or there is no
    /// replica yet).
    pub fn changes_since(&self, since: &[u8]) -> Value {
        let Some(kdm) = self.session.kdm() else { return json!({ "changes": null, "unavailable": "the project has no replica yet" }) };
        let replica = kdm.replica();
        let since = if since.is_empty() { Ok(Version::empty()) } else { Version::decode(since) };
        match since.and_then(|since| replica.changes_since(&since)) {
            Ok(changes) => json!({ "changes": changes.to_recorded_json(replica.schema()) }),
            Err(error) => json!({ "changes": null, "unavailable": error.to_string() }),
        }
    }

    /// An agent's reply (JSON text) with every object id it holds as a value or a key named as
    /// the tool interface names it (its tool id, KitObjectModel OM9); unchanged without a replica.
    /// Text that merely holds an id stays as it is, and the project keeps its own ids.
    ///
    /// The editor sends every `ai/inspect`, `document/check` and `ai/run` reply through here, and
    /// Motion's replies name items, layers and Render Queue items by the engine's own ids, which
    /// are their tool ids: what is left to name is an object id the replica made (a hyphenated
    /// UUID). A reply holding none comes back as it is, only checked to be JSON; otherwise only the
    /// objects it names are read (`Replica::tool_ids_of`), never the whole document's. The
    /// project object's id (`project`) is a word a reply also uses as a key, so it is never taken
    /// for an object.
    pub fn model_reply(&self, value: &str) -> Result<String, String> {
        let not_json = |error: serde_json::Error| format!("the reply is not JSON: {error}");
        let kdm = self.session.kdm();
        if kdm.is_none() || !holds_object_id(value) {
            serde_json::from_str::<serde::de::IgnoredAny>(value).map_err(not_json)?;
            return Ok(value.to_owned());
        }
        let parsed: Value = serde_json::from_str(value).map_err(not_json)?;
        let Some(replica) = kdm.as_ref().map(|kdm| kdm.replica()) else { return Ok(value.to_owned()) };
        let mut named = BTreeSet::new();
        named_objects(&parsed, replica, &mut named);
        if named.is_empty() {
            return Ok(value.to_owned());
        }
        Ok(replica.tool_ids_of(&named).show_json(&parsed).to_string())
    }

    /// `model/describe` as JSON: the object model over the project's change exchange.
    pub fn model_describe(&mut self) -> Result<Value, Value> {
        Exchange::new().describe(self).map_err(|failure| failure.to_json())
    }

    /// `model/batch`: one batch of the object model (reads, writes, commands) as one undo step; a
    /// failure is the error's JSON (`{"code", "index", "message"}`).
    pub fn model_batch(&mut self, request: &Value) -> Result<Value, Value> {
        let answer = Exchange::new().batch(self, request, false).map_err(|failure| failure.to_json());
        self.lend_footage();
        answer
    }
}

/// Whether `text` is an object id the replica made: a hyphenated, lower-case UUID.
fn is_object_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(at, byte)| if matches!(at, 8 | 13 | 18 | 23) { *byte == b'-' } else { matches!(*byte, b'0'..=b'9' | b'a'..=b'f') })
}

/// Whether `text` holds an object id the replica made anywhere.
fn holds_object_id(text: &str) -> bool {
    text.as_bytes().windows(36).any(|window| std::str::from_utf8(window).is_ok_and(is_object_id))
}

/// The objects of `replica` that `value` names: a string, a key, or a `:`-separated part of a key
/// that is an object id the replica made and holds.
fn named_objects(value: &Value, replica: &Replica, out: &mut BTreeSet<ObjectId>) {
    let mut name = |text: &str| {
        if is_object_id(text) {
            let id = ObjectId::from_string(text);
            if replica.exists(&id) {
                out.insert(id);
            }
        }
    };
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            Value::String(text) => name(text),
            Value::Array(items) => stack.extend(items),
            Value::Object(members) => {
                for (key, member) in members {
                    key.split(':').for_each(&mut name);
                    stack.push(member);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use kition_kit::KitDocument;
    use kition_kit::kdm::{ActorId, Replica};
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

    fn person() -> Origin {
        Origin::person("person:local")
    }

    /// The active composition's layer names, top first.
    fn layer_names(doc: &MotionDocument) -> Vec<String> {
        doc.session.active_comp().map(|comp| comp.layers.iter().map(|layer| layer.name.clone()).collect()).unwrap_or_default()
    }

    /// A solid layer named `name` added to the active composition, run as `origin`.
    fn add_solid(doc: &mut MotionDocument, name: &str, origin: &Origin) {
        doc.execute_as("layer.newSolid", json!({ "name": name, "color": "#00ff00" }), origin).unwrap();
    }

    /// A draft of `doc` as the editor opens one: an engine of its own on the document's replica
    /// (no file), under a fresh actor. Answers the draft and the version it was made at.
    fn draft_of(doc: &mut MotionDocument) -> (MotionDocument, Vec<u8>) {
        let base = doc.version().unwrap();
        let draft = MotionDocument::open_with_replica("Motion.kmotion", &[], &doc.snapshot().unwrap(), &[]).unwrap();
        (draft, base)
    }

    /// The object ids the replica holds, sorted.
    fn object_ids(doc: &mut MotionDocument) -> Vec<String> {
        let mut ids: Vec<String> = doc.replicated_state().unwrap()["objects"].as_object().unwrap().keys().cloned().collect();
        ids.sort();
        ids
    }

    /// The project as the engine holds it, its id counter left out (it never goes back within a
    /// session: an id handed out stays used, also after an undo): what the engine's view and a
    /// read of the replica agree on.
    fn view(doc: &MotionDocument) -> Value {
        serde_json::to_value(kdm_model::normalize(&doc.session.project)).unwrap()
    }

    /// The whole state: the replica's (its hash) and the engine's view of it.
    fn whole(doc: &mut MotionDocument) -> (String, Value) {
        (doc.session.state_hash().unwrap(), view(doc))
    }

    /// A draft merges in as one step of the person's Undo: the comparison beforehand names what
    /// the merge does and changes nothing, the engine's project follows the merge, an edit made
    /// here meanwhile stays, and a refused merge changes nothing.
    #[test]
    fn a_draft_merges_in_as_one_undo_step_and_the_project_follows() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let (mut draft, base) = draft_of(&mut doc);
        add_solid(&mut draft, "From the draft", &person());
        let changes = draft.export_changes(&base).unwrap();
        add_solid(&mut doc, "Meanwhile", &person());
        let before = layer_names(&doc);
        let version = doc.version().unwrap();
        let preview = doc.compare_changes(&changes).unwrap();
        assert_eq!(doc.version().unwrap(), version, "comparing changes nothing");
        assert_eq!(layer_names(&doc), before);
        let answer = doc.merge_changes(&changes, &person()).unwrap();
        assert_eq!(answer["summary"], preview, "the comparison named what the merge did: {answer}");
        assert_ne!(answer["damage"], json!("none"), "{answer}");
        assert_eq!(answer["scope"], json!("person"), "one step of the person's: {answer}");
        let merged = layer_names(&doc);
        assert!(merged.contains(&"From the draft".to_string()), "the engine's project follows the merge: {merged:?}");
        assert!(merged.contains(&"Meanwhile".to_string()), "the edit made here stays: {merged:?}");
        assert_eq!(merged.len(), before.len() + 1);
        assert!(doc.dirty());
        assert!(doc.undo().unwrap().0);
        assert_eq!(layer_names(&doc), before, "one undo takes the whole draft back, and only the draft");
        assert!(doc.redo().unwrap().0);
        assert_eq!(layer_names(&doc), merged, "one redo brings it back");
        let at = doc.version().unwrap();
        assert!(doc.merge_changes(b"not changes", &person()).is_err());
        assert!(doc.compare_changes(b"not changes").is_err());
        assert_eq!(doc.version().unwrap(), at, "a refused merge changes nothing");
        assert_eq!(doc.session.kdm_resets(), 0);
    }

    /// KitModel M12a: an agent's draft, merged with the agent's origin as the editor merges it
    /// (`draft/merge` records `{kind: "agent"}`), is one step of the person's Undo. The document
    /// added a layer to the composition meanwhile and the draft another: both are there. The
    /// agent's Undo does not take the merge back, the person's does, to the exact state before
    /// (the replica's and the engine's), and Redo brings the exact merged state back; the
    /// replica's histories say the same as the engine's.
    #[test]
    fn an_agents_draft_is_one_step_of_the_persons_undo_and_undoes_exactly() {
        let agent = Origin::agent("agent:test");
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let (mut draft, base) = draft_of(&mut doc);
        add_solid(&mut draft, "Agent's", &agent);
        let changes = draft.export_changes(&base).unwrap();
        add_solid(&mut doc, "Person's", &person());
        let before = whole(&mut doc);
        let names = layer_names(&doc);
        let history = doc.session.history.undo.len();

        let answer = doc.merge_changes(&changes, &agent).unwrap();
        assert_eq!(answer["scope"], json!("person"), "an agent's draft is the person's step: {answer}");
        let merged = whole(&mut doc);
        let now = layer_names(&doc);
        assert_eq!(now.len(), names.len() + 1, "both layers added to the composition are there: {now:?}");
        assert!(names.iter().all(|name| now.contains(name)) && now.contains(&"Agent's".to_string()), "{now:?}");
        assert_eq!(doc.session.history.undo.len(), history + 1, "one step of the person's history");
        assert_eq!(doc.session.history.undo.last().map(|entry| entry.0.as_str()), Some("Merge Draft"));
        {
            let kdm = doc.session.kdm().unwrap();
            assert!(kdm.can_undo_in(UndoScope::Person), "the replica records it in the person's history");
            assert!(!kdm.can_undo_in(UndoScope::Agent), "and nothing in the agents'");
        }

        let undone = doc.execute_as("edit.undo", json!({ "scope": "agent" }), &agent).unwrap();
        assert_eq!(undone["undone"], Value::Null, "the agent's Undo has nothing to take back: {undone}");
        assert_eq!(whole(&mut doc), merged, "the merge stays");
        assert!(doc.undo().unwrap().0);
        assert_eq!(whole(&mut doc), before, "one Undo of the person: exactly the state before the merge");
        assert_eq!(layer_names(&doc), names);
        assert!(doc.redo().unwrap().0);
        assert_eq!(whole(&mut doc), merged, "one Redo: exactly the merged state");
        assert_eq!(doc.session.kdm_resets(), 0);
    }

    /// An earlier state (a checkpoint's snapshot) is written over the project by object id as one
    /// undo step: the replica holds that state again, with the same ids, Undo takes it back to the
    /// exact state before and Redo brings the exact restored state again.
    #[test]
    fn a_restored_state_keeps_the_ids_and_undoes_and_redoes_exactly() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        doc.version().unwrap();
        let checkpoint = doc.snapshot().unwrap();
        let names = layer_names(&doc);
        add_solid(&mut doc, "Since", &person());
        let red = doc.session.active_comp().and_then(|comp| comp.layers.iter().find(|layer| layer.name == "Red")).map(|layer| layer.id.0).unwrap();
        doc.execute("layer.rename", json!({ "layer": red, "name": "Renamed" })).unwrap();
        let before = whole(&mut doc);
        let damage = doc.restore_state(&checkpoint, &person()).unwrap();
        assert_ne!(damage, Damage::None);
        assert_eq!(layer_names(&doc), names, "the engine's project follows the restore");
        let schema = <kdm_model::MotionModel as kition_kit::session::KitModel>::schema();
        let then = Replica::load(schema, &checkpoint, ActorId::random()).unwrap().state_json().unwrap();
        assert_eq!(doc.replicated_state().unwrap()["objects"], then["objects"], "the checkpoint's objects, under their ids");
        let restored = whole(&mut doc);
        let at = doc.version().unwrap();
        assert_eq!(doc.restore_state(&checkpoint, &person()).unwrap(), Damage::None, "the same state again changes nothing");
        assert_eq!(doc.version().unwrap(), at);
        assert!(doc.undo().unwrap().0);
        assert_eq!(whole(&mut doc), before, "one Undo: exactly the state before the restore");
        assert!(doc.redo().unwrap().0);
        assert_eq!(whole(&mut doc), restored, "one Redo: exactly the restored state");
        let other = MotionDocument::open("Motion.kmotion", &fixture()).unwrap().snapshot().unwrap();
        assert!(doc.restore_state(&other, &person()).is_err(), "another document's state is refused");
        assert_eq!(doc.session.kdm_resets(), 0);
    }

    /// The receipt of what changed since a version, or why there is none.
    #[test]
    fn changes_since_answers_a_receipt_or_why_not() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert!(doc.changes_since(&[])["changes"].is_null(), "no replica yet");
        let since = doc.version().unwrap();
        let red = doc.session.active_comp().and_then(|comp| comp.layers.first()).map(|layer| layer.id.0).unwrap();
        doc.execute("layer.rename", json!({ "layer": red, "name": "Receipt" })).unwrap();
        let receipt = doc.changes_since(&since);
        assert!(receipt["changes"].is_object(), "{receipt}");
        assert!(receipt.to_string().contains("Receipt"), "the edit is in the receipt: {receipt}");
        let now = doc.version().unwrap();
        assert_ne!(receipt["changes"], doc.changes_since(&now)["changes"]);
        let other = MotionDocument::open("Motion.kmotion", &fixture()).unwrap().version().unwrap();
        let refused = doc.changes_since(&other);
        assert!(refused["changes"].is_null() && refused["unavailable"].is_string(), "{refused}");
    }

    /// An agent's reply names objects by their tool ids (a layer by its layer id); text that
    /// merely holds an object id stays, and so does a reply naming no object, byte for byte.
    #[test]
    fn a_reply_names_objects_by_their_tool_ids() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        assert_eq!(doc.model_reply(r#"{"a":1}"#).unwrap(), r#"{"a":1}"#, "without a replica the reply is as it is");
        let state = doc.replicated_state().unwrap();
        let object = state["objects"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(id, object)| object["$type"] == "Layer" && is_object_id(id))
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| panic!("a layer object: {state}"));
        let full = doc.session.kdm().unwrap().replica().tool_ids();
        let reply = json!({ "target": object, "note": format!("about {object}") });
        let shown: Value = serde_json::from_str(&doc.model_reply(&reply.to_string()).unwrap()).unwrap();
        assert_eq!(shown, full.show_json(&reply), "{shown}");
        assert_ne!(shown["target"], json!(object), "the layer is named by its tool id: {shown}");
        assert_eq!(shown["note"], json!(format!("about {object}")), "text that holds an id stays");
        let none = "{ \"project\" : { \"name\": \"Motion\" },\n  \"layers\": [10, 11] }";
        assert_eq!(doc.model_reply(none).unwrap(), none, "a reply naming no object comes back as it is");
        assert!(doc.model_reply("not json").is_err());
    }

    /// `modelReply` reads only the objects a reply names: in a project of many objects, a reply
    /// naming one object shows it under the tool id a read of the whole project gives it, for
    /// every object.
    #[test]
    fn a_reply_shows_the_ids_the_full_read_gives() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        for at in 0..6 {
            add_solid(&mut doc, &format!("Solid {at}"), &person());
        }
        let state = doc.replicated_state().unwrap();
        let objects: Vec<String> = state["objects"].as_object().unwrap().keys().filter(|id| is_object_id(id)).cloned().collect();
        assert!(objects.len() > 60, "many objects: {}", objects.len());
        let full = doc.session.kdm().unwrap().replica().tool_ids();
        for object in &objects {
            let reply = json!({ "target": object, "list": [object], "note": "one object" });
            let shown: Value = serde_json::from_str(&doc.model_reply(&reply.to_string()).unwrap()).unwrap();
            assert_eq!(shown, full.show_json(&reply), "{object}");
        }
    }

    /// The object model over the project: its types, a read, and a command run as one batch that
    /// the project follows and one Undo takes back.
    #[test]
    fn the_object_model_describes_reads_and_writes() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        let described = doc.model_describe().unwrap();
        for ty in ["Project", "Item", "Layer"] {
            assert!(described["types"].get(ty).is_some(), "{ty} in {}", described["types"]);
        }
        let read = doc.model_batch(&json!({ "ops": [], "load": [{ "target": "root" }] })).unwrap();
        assert!(read["loaded"][0]["fields"].is_object(), "{read}");
        let before = layer_names(&doc);
        let origin = json!({ "kind": "person", "principal": "person:local" });
        doc.model_batch(&json!({ "origin": origin, "ops": [{ "op": "command", "id": "layer.newNull", "params": {} }] })).unwrap();
        assert_eq!(layer_names(&doc).len(), before.len() + 1, "the batch's command ran on the project");
        assert!(doc.undo().unwrap().0);
        assert_eq!(layer_names(&doc), before, "one undo takes the batch back");
        assert!(doc.model_batch(&json!({ "ops": [{ "op": "no such op" }] })).is_err());
    }

    /// An unchanged project saves the bytes it was opened from, and reopened with its stored
    /// replica it holds the same objects under the same ids, writing nothing.
    #[test]
    fn an_unchanged_project_saves_its_own_bytes_and_reopens_with_the_same_ids() {
        let bytes = fixture();
        let mut doc = MotionDocument::open("Motion.kmotion", &bytes).unwrap();
        let ids = object_ids(&mut doc);
        assert_eq!(doc.save(None).unwrap(), bytes, "an unchanged project saves byte-identical, also with a replica made");
        let version = doc.version().unwrap();
        let snapshot = doc.snapshot().unwrap();
        let mut again = MotionDocument::open_with_replica("Motion.kmotion", &bytes, &snapshot, &[]).unwrap();
        assert_eq!(again.version().unwrap(), version, "reopening writes nothing");
        assert_eq!(object_ids(&mut again), ids, "the same object ids");
        assert!(!again.dirty());
        assert_eq!(again.save(None).unwrap(), bytes, "and saves the same bytes again: the new actor's id range is not written");
    }

    /// A file holds no session's state: two sessions that open one file, with its stored replica
    /// or plainly, save the bytes it holds; and making the same edit, one that mints no id, they
    /// save the same bytes again.
    #[test]
    fn two_sessions_that_open_one_file_save_the_same_bytes() {
        let mut doc = MotionDocument::open("Motion.kmotion", &fixture()).unwrap();
        add_solid(&mut doc, "Green", &person());
        let bytes = doc.save(None).unwrap();
        let snapshot = doc.snapshot().unwrap();
        let mut one = MotionDocument::open_with_replica("Motion.kmotion", &bytes, &snapshot, &[]).unwrap();
        let mut two = MotionDocument::open_with_replica("Motion.kmotion", &bytes, &snapshot, &[]).unwrap();
        let mut plain = MotionDocument::open("Motion.kmotion", &bytes).unwrap();
        assert_ne!(one.session.kdm().unwrap().replica().actor(), two.session.kdm().unwrap().replica().actor(), "each session is its own actor");
        for (name, session) in [("one", &mut one), ("two", &mut two), ("plain", &mut plain)] {
            assert_eq!(session.save(None).unwrap(), bytes, "{name} saves the bytes it opened");
        }
        for session in [&mut one, &mut two] {
            let red = session.session.active_comp().and_then(|comp| comp.layers.iter().find(|layer| layer.name == "Red")).map(|layer| layer.id.0).unwrap();
            session.execute("layer.rename", json!({ "layer": red, "name": "Same" })).unwrap();
        }
        let edited = one.save(None).unwrap();
        assert_ne!(edited, bytes);
        assert_eq!(two.save(None).unwrap(), edited, "the same edit saves the same bytes");
        let reopened = MotionDocument::open("Motion.kmotion", &edited).unwrap().save(None).unwrap();
        assert_eq!(reopened, edited);
    }
}
