//! A text animator's selectors and animated properties are removed one at a time (`prop.removeGroup`).

use serde_json::{Value, json};

use crate::Session;

fn find<'a>(tree: &'a Value, matching: &dyn Fn(&Value) -> bool) -> Option<&'a Value> {
    if matching(tree) {
        return Some(tree);
    }
    tree["children"].as_array()?.iter().find_map(|child| find(child, matching))
}

fn animator(s: &mut Session, layer: u64, uid: u64) -> Value {
    let tree = s.execute("layer.tree", json!({"layer": layer, "depth": 12})).unwrap();
    find(&tree["properties"], &|node| node["uid"] == uid).cloned().unwrap_or(Value::Null)
}

fn names(group: &Value, holder: &str) -> Vec<String> {
    group["children"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|g| g["match"] == holder)
        .flat_map(|g| g["children"].as_array().cloned().unwrap_or_default())
        .map(|c| c["match"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn a_selector_or_a_property_of_an_animator_is_removed_alone() {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "T", "width": 320, "height": 180, "frameRate": 30, "duration": 2})).unwrap();
    let layer = s.execute("layer.newText", json!({"text": "Hello"})).unwrap()["layer"].as_u64().unwrap();
    let uid = s.execute("layer.addTextAnimator", json!({"layer": layer, "properties": ["position"]})).unwrap()["animator"].as_u64().unwrap();
    s.execute("layer.addTextAnimatorProperty", json!({"layer": layer, "animator": uid, "property": "scale"})).unwrap();
    s.execute("layer.addTextSelector", json!({"layer": layer, "animator": uid, "kind": "wiggly"})).unwrap();
    let group = animator(&mut s, layer, uid);
    assert_eq!(names(&group, "properties"), ["position", "scale"]);
    assert_eq!(names(&group, "selectors"), ["rangeSelector", "wigglySelector"]);
    let scale = find(&group, &|node| node["match"] == "scale").unwrap()["uid"].as_u64().unwrap();
    let wiggly = find(&group, &|node| node["match"] == "wigglySelector").unwrap()["uid"].as_u64().unwrap();
    s.execute("prop.removeGroup", json!({"layer": layer, "prop": scale})).unwrap();
    s.execute("prop.removeGroup", json!({"layer": layer, "prop": wiggly})).unwrap();
    let group = animator(&mut s, layer, uid);
    assert_eq!(names(&group, "properties"), ["position"]);
    assert_eq!(names(&group, "selectors"), ["rangeSelector"]);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(names(&animator(&mut s, layer, uid), "selectors"), ["rangeSelector", "wigglySelector"], "one undo puts the selector back");
    // A fixed group (the animator's Properties) or something that is not there: refused, nothing changes.
    let holder = find(&animator(&mut s, layer, uid), &|node| node["match"] == "properties").unwrap()["uid"].as_u64().unwrap();
    let revision = s.revision;
    assert!(s.execute("prop.removeGroup", json!({"layer": layer, "prop": holder})).is_err());
    assert!(s.execute("prop.removeGroup", json!({"layer": layer, "prop": 999_999})).is_err());
    assert!(s.execute("prop.removeGroup", json!({"layer": layer})).is_err());
    assert_eq!(s.revision, revision);
}
