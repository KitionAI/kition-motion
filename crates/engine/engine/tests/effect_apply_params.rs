//! `effect.apply` takes the effect's settings in the same step and answers its parameters: in
//! khp-bench's video runs an effect took a call to apply, another to learn its parameter names and
//! one more to set them, and agents mostly left effects out.
use kition_motion_engine::Session;
use serde_json::{Value, json};

fn solid() -> (Session, u64) {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Params", "width": 192, "height": 108, "duration": 2, "frameRate": 30})).unwrap();
    let layer = s.execute("layer.newSolid", json!({"name": "BG", "color": "#101828"})).unwrap()["layer"].as_u64().unwrap();
    (s, layer)
}

/// Effect instances on `layer`.
fn effects(s: &mut Session, layer: u64) -> usize {
    let tree = s.execute("prop.get", json!({"layer": layer, "path": "effects"})).unwrap();
    tree["children"].as_array().map_or(0, Vec::len)
}

#[test]
fn params_are_set_in_the_same_step_and_listed() {
    let (mut s, layer) = solid();
    let r = s.execute("effect.apply", json!({"layers": [layer], "effect": "Gaussian Blur", "params": {"Blurriness": 30}})).unwrap();
    assert_eq!(effects(&mut s, layer), 1);
    let path = r["paths"][0].as_str().unwrap().to_owned();
    let listed: Vec<&Value> = r["params"].as_array().unwrap().iter().collect();
    let blur = listed.iter().find(|p| p["name"].as_str() == Some("Blurriness")).expect("Blurriness is listed");
    assert_eq!(blur["value"], json!(30.0), "{r}");
    let read = s.execute("prop.get", json!({"layer": layer, "path": blur["path"]})).unwrap();
    assert_eq!(read["value"], json!(30.0), "{read}");
    assert!(blur["path"].as_str().unwrap().starts_with(&path));
}

#[test]
fn a_parameter_name_without_spaces_or_case_is_found() {
    let (mut s, layer) = solid();
    let r =
        s.execute("effect.apply", json!({"layers": [layer], "effect": "Gradient Ramp", "params": {"startcolor": "#1d4ed8", "end color": "#020617"}})).unwrap();
    let start = r["params"].as_array().unwrap().iter().find(|p| p["name"].as_str() == Some("Start Color")).unwrap()["value"].clone();
    let rgb: Vec<f64> = start.as_array().unwrap().iter().take(3).map(|v| v.as_f64().unwrap()).collect();
    assert!((rgb[2] - 0xd8 as f64 / 255.0).abs() < 0.01 && rgb[0] < 0.2, "{start}");
}

#[test]
fn an_unknown_parameter_changes_nothing_and_names_the_right_ones() {
    let (mut s, layer) = solid();
    let before = effects(&mut s, layer);
    let err = s.execute("effect.apply", json!({"layers": [layer], "effect": "Gaussian Blur", "params": {"radius": 30}})).unwrap_err().to_string();
    assert!(err.contains("no parameter `radius`") && err.contains("Blurriness"), "{err}");
    assert_eq!(effects(&mut s, layer), before, "the effect was applied although a parameter was wrong");
}

#[test]
fn a_layer_style_takes_its_settings_in_the_same_step() {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Glow", "width": 192, "height": 108, "duration": 2, "frameRate": 30})).unwrap();
    let text = s.execute("layer.newText", json!({"text": "3"})).unwrap()["layer"].as_u64().unwrap();
    let r = s.execute("layer.style.outerGlow", json!({"layers": [text], "params": {"size": 40, "color": "#2f7bff", "opacity": 90}})).unwrap();
    let get = |name: &str| r["params"].as_array().unwrap().iter().find(|p| p["name"].as_str() == Some(name)).unwrap()["value"].clone();
    assert_eq!(get("Size"), json!(40.0), "{r}");
    assert_eq!(get("Opacity"), json!(90.0), "{r}");
    let err = s.execute("layer.style.dropShadow", json!({"layers": [text], "params": {"blur": 12}})).unwrap_err().to_string();
    assert!(err.contains("no setting `blur`") && err.contains("Size"), "{err}");
    let styles = s.execute("layer.style.list", json!({"layer": text})).unwrap();
    let added: Vec<&str> = styles["styles"].as_array().unwrap().iter().filter_map(|x| x["style"].as_str()).collect();
    assert_eq!(added, vec!["outerGlow"], "a refused style was added: {styles}");
}

/// The value of the parameter `name` in `effect.apply`'s answer.
fn listed(r: &Value, name: &str) -> Value {
    r["params"].as_array().unwrap().iter().find(|p| p["name"].as_str() == Some(name)).map(|p| p["value"].clone()).unwrap_or(Value::Null)
}

#[test]
fn an_effect_starts_at_after_effects_defaults_and_reset_returns_there() {
    let (mut s, layer) = solid();
    let r = s.execute("effect.apply", json!({"layers": [layer], "effect": "Gaussian Blur"})).unwrap();
    assert_eq!(listed(&r, "Blurriness"), json!(0.0), "{r}");
    // Reset gives the value it started with.
    let path = format!("{}/blurriness", r["paths"][0].as_str().unwrap());
    s.execute("prop.set", json!({"layer": layer, "path": path, "value": 3})).unwrap();
    s.execute("prop.reset", json!({"layer": layer, "path": path})).unwrap();
    assert_eq!(s.execute("prop.get", json!({"layer": layer, "path": path})).unwrap()["value"], json!(0.0));
}

#[test]
fn a_starting_value_never_overrides_params() {
    let (mut s, layer) = solid();
    let r = s.execute("effect.apply", json!({"layers": [layer], "effect": "Gaussian Blur", "params": {"blurriness": 4}})).unwrap();
    assert_eq!(listed(&r, "Blurriness"), json!(4.0), "{r}");
}

#[test]
fn an_effect_and_its_settings_are_one_undo_step() {
    let (mut s, layer) = solid();
    s.execute("effect.apply", json!({"layers": [layer], "effect": "Gradient Ramp", "params": {"Start Color": "#1d4ed8", "shape": 1}})).unwrap();
    s.execute("layer.style.add", json!({"layers": [layer], "style": "outerGlow", "params": {"size": 40, "opacity": 90}})).unwrap();
    s.execute("edit.undo", json!({})).unwrap();
    let styles = s.execute("layer.style.list", json!({"layer": layer})).unwrap();
    assert!(styles["styles"].as_array().is_none_or(Vec::is_empty), "{styles}");
    assert_eq!(effects(&mut s, layer), 1);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(effects(&mut s, layer), 0);
}
