//! The editor state's members by the reads commands declare (kition-sdk
//! `docs/platform/kits/call-context.md`: `selection`, `part`, `time`, `mode`, `active`, `area`).
//! The premise guard compares a command's implicit read by the member of that name in the editor
//! state before and after another writer's change: equal applies, different refuses as `changed`
//! on that read, and a member the state lacks refuses any change as `unknown`.
//!
//! A member must hold every field of the snapshot ([`kition_motion_engine::context::CallState`])
//! that a command declaring that read may consult and no call context sets. `tests/acts.rs`'s scan
//! of the engine finds commands whose code reads more than they declare (the Project panel's
//! selection, the 3D view, the region of interest, the open comps, the layer selection with only
//! `time` or `mode` declared), so each member is the whole snapshot except the fields left out
//! below ([`OUTSIDE`]); a new field of the snapshot is in every member until it is placed:
//! - the current times are outside `selection`, `mode`, `active` and `area`: a person moving the
//!   time does not refuse a step that gives its time (`at.time`) or reads none, so every command
//!   whose code reads the time declares `time` (`edit.copy`, `edit.selectAll` and `learn.step`
//!   among them).
//! - the active comp is outside `time`: a step reading the time either gives its part (`at.part`
//!   sets the active comp) or reads `part`, which holds it.
//!
//! The snapshot's own fields stay at the top level beside the members; restoring reads only them.

use serde_json::{Map, Value};

/// The members, as kition-sdk's `EditorState` names them, each with the snapshot's fields
/// (`CallState`, camelCase) it leaves out.
const OUTSIDE: &[(&str, &[&str])] =
    &[("part", &[]), ("time", &["activeComp"]), ("selection", &["times"]), ("mode", &["times"]), ("active", &["times"]), ("area", &["times"])];

/// Whether `key` is one of the members.
fn is_member(key: &str) -> bool {
    OUTSIDE.iter().any(|(member, _)| *member == key)
}

/// `snapshot` (the serialized `CallState`) with its members added.
pub fn with_members(snapshot: Value) -> Value {
    let Value::Object(fields) = snapshot else { return snapshot };
    let mut state = fields.clone();
    for (member, outside) in OUTSIDE {
        let held: Map<String, Value> =
            fields.iter().filter(|(field, _)| !outside.contains(&field.as_str())).map(|(field, value)| (field.clone(), value.clone())).collect();
        state.insert((*member).to_owned(), Value::Object(held));
    }
    Value::Object(state)
}

/// The snapshot's own fields of `state`, without the members [`with_members`] added.
pub fn snapshot(state: &Value) -> Value {
    match state {
        Value::Object(fields) => Value::Object(fields.iter().filter(|(key, _)| !is_member(key)).map(|(key, value)| (key.clone(), value.clone())).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kition_motion_engine::context::CallState;

    #[test]
    fn members_are_declared_reads_and_leave_out_fields_of_the_snapshot() {
        let saved = serde_json::to_value(CallState::default()).unwrap();
        let fields = saved.as_object().unwrap();
        assert_eq!(OUTSIDE.len(), 6);
        for (member, outside) in OUTSIDE {
            assert!(serde_json::from_value::<kition_kit::EditorState>(serde_json::json!(member)).is_ok(), "{member} is no read a command declares");
            assert!(!fields.contains_key(*member), "{member} is also a field of the snapshot");
            for field in *outside {
                assert!(fields.contains_key(*field), "{member} leaves out {field}, which the snapshot does not have: {saved}");
            }
        }
    }

    #[test]
    fn members_are_added_beside_the_snapshot_and_taken_off_again() {
        let saved = serde_json::to_value(CallState::default()).unwrap();
        let state = with_members(saved.clone());
        let count = saved.as_object().unwrap().len();
        assert_eq!(state["part"], saved);
        assert!(state["time"].get("times").is_some() && state["time"].get("activeComp").is_none());
        for member in ["selection", "mode", "active", "area"] {
            assert!(state[member].get("times").is_none(), "{member}");
            assert_eq!(state[member].as_object().unwrap().len(), count - 1, "{member}");
        }
        assert_eq!(snapshot(&state), saved);
    }

    #[test]
    fn a_field_no_member_leaves_out_is_in_every_member() {
        let state = with_members(serde_json::json!({ "newField": 1 }));
        for (member, _) in OUTSIDE {
            assert_eq!(state[*member]["newField"], 1, "{member}");
        }
    }
}
