//! An agent's parameters that contradict a command's schema are refused before the command runs
//! (KHP A18; [`ReplicaPort::model_command`](kition_kit::session::model::ReplicaPort), the agent's
//! way in):
//! the engine's readers take a value of the wrong type as absent and fall back to a default, so
//! `layer.newSolid {width: "wide"}` would otherwise make a solid of the default size. Only the
//! types a schema states are checked (`string`, `number`, `integer`, `boolean`, `array`, `object`,
//! `file`); `json` and untyped keys take anything, and `null` stands for "none" everywhere.

use std::collections::HashMap;
use std::sync::OnceLock;

use kition_kit::KitCommand;

use crate::TypedFrom;
use serde_json::Value;

/// Each command's parameter schema, as `commands()` lists it: typed from the kit's table too, so
/// this check and the one the agent's tools make agree (khp-bench r3: a shape's `size` passed one
/// and was refused by the other).
fn schemas() -> &'static HashMap<&'static str, Value> {
    static SCHEMAS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    SCHEMAS.get_or_init(|| {
        kition_motion_engine::commands::command_specs()
            .iter()
            .filter_map(|spec| {
                let command = KitCommand { id: spec.id.to_owned(), params: Some(spec.params.to_owned()), ..Default::default() }
                    .with_kit_schema()
                    .typed_from(crate::param_types());
                command.schema.map(|mut schema| {
                    crate::param_keys::ids_as_text(&mut schema);
                    (spec.id, schema)
                })
            })
            .collect()
    })
}

fn fits(value: &Value, kind: &str) -> bool {
    match kind {
        "string" | "file" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some() || value.as_f64().is_some_and(|n| n.fract() == 0.0),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => true,
    }
}

/// An error naming the first parameter whose value contradicts the schema's type.
pub(crate) fn check(command: &str, params: &Value) -> Result<(), String> {
    let (Some(Value::Object(schema)), Value::Object(given)) = (schemas().get(command), params) else { return Ok(()) };
    for (key, value) in given {
        if value.is_null() {
            continue;
        }
        let Some(kind) = schema.get(key).and_then(|node| node.get("type")).and_then(Value::as_str) else { continue };
        if !fits(value, kind) {
            return Err(format!(
                "invalid parameters for `{command}`: `{key}` must be {} {kind}, not {value}",
                if kind.starts_with(['a', 'i', 'o']) { "an" } else { "a" }
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::MotionDocument;
    use kition_kit::KitDocument;
    use kition_kit::kdm::Origin;
    use kition_kit::session::model::ReplicaPort;
    use serde_json::json;

    #[test]
    fn a_value_of_the_wrong_type_is_refused_and_changes_nothing() {
        let mut document = MotionDocument::create("Title.kmotion", 320, 180).expect("project");
        let before = document.save(None).expect("save");
        let bad = json!({ "name": { "khp-check": "not a valid value" }, "color": { "khp-check": "not a valid value" } });
        let agent = Origin::agent("person:local");
        let error = document.model_command("layer.newSolid", bad, &agent).expect_err("refused");
        assert!(error.contains("`name` must be a string") || error.contains("`color` must be a string"), "{error}");
        assert_eq!(document.save(None).expect("save"), before, "nothing changed");
        document.model_command("layer.newSolid", json!({ "name": "Red", "color": "#ff0000" }), &agent).expect("a right call runs");
    }
}
