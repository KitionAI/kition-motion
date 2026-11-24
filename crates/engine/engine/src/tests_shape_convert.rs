//! A shape layer's Rectangle, Ellipse and Polystar Paths convert to editable Bezier paths, and the
//! vertex commands find a shape Path item by its uid even when that uid is also a valid 1-based
//! mask index on the layer.

use serde_json::{Value, json};

use crate::Session;

fn comp() -> Session {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Test", "width": 640, "height": 360, "frameRate": 30, "duration": 4})).unwrap();
    s
}

/// Every node of a `layer.tree` answer whose `match` is `kind`.
fn all<'a>(tree: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if tree["match"] == kind {
        out.push(tree);
    }
    for child in tree["children"].as_array().into_iter().flatten() {
        all(child, kind, out);
    }
}

fn items(s: &mut Session, layer: u64, kind: &str) -> Vec<Value> {
    let tree = s.execute("layer.tree", json!({"layer": layer, "depth": 12})).unwrap();
    let mut out = Vec::new();
    all(&tree["properties"], kind, &mut out);
    out.into_iter().cloned().collect()
}

fn path_of(s: &mut Session, layer: u64, uid: u64) -> Value {
    s.execute("prop.get", json!({"layer": layer, "path": format!("@{uid}")})).map(|v| v["value"]["v"].clone()).unwrap_or(Value::Null)
}

#[test]
fn a_shape_path_whose_uid_is_a_mask_index_is_edited_not_the_mask() {
    let mut s = Session::default();
    // The shape layer is made first, so its Path item's uid is small; masks added after it get
    // larger uids, and enough of them make the Path's uid a valid 1-based mask index too.
    s.execute("comp.new", json!({"name": "Test", "width": 640, "height": 360, "frameRate": 30, "duration": 4})).unwrap();
    let layer = s.execute("layer.newShape", json!({"kind": "rect"})).unwrap()["layer"].as_u64().unwrap();
    s.execute("path.convertToBezier", json!({"layer": layer})).unwrap();
    let path = items(&mut s, layer, "path").into_iter().find(|item| item["children"].is_array()).expect("a Path item");
    let uid = path["uid"].as_u64().unwrap();
    let inner = path["children"].as_array().unwrap().iter().find(|child| child["match"] == "path").unwrap()["uid"].as_u64().unwrap();
    for _ in 0..uid {
        s.execute("mask.new", json!({"layer": layer, "vertices": [[0, 0], [10, 0], [10, 10]]})).unwrap();
    }
    let masks = items(&mut s, layer, "masks")[0]["children"].as_array().unwrap().len() as u64;
    assert!(uid <= masks, "the Path item's uid {uid} is a mask index (of {masks})");
    let mask_before = s.execute("layer.tree", json!({"layer": layer, "depth": 4})).unwrap()["properties"]["children"].to_string();
    let before = path_of(&mut s, layer, inner);
    s.execute("mask.setVertex", json!({"layer": layer, "mask": uid, "index": 0, "point": [-200, -150]})).unwrap();
    let after = path_of(&mut s, layer, inner);
    assert_ne!(before, after, "the shape path moved its vertex");
    assert_eq!(after["vertices"][0], json!([-200.0, -150.0]), "{after}");
    let masks_after = s.execute("layer.tree", json!({"layer": layer, "depth": 4})).unwrap()["properties"]["children"].to_string();
    let only_masks = |tree: &str| -> String { tree.split("\"masks\"").nth(1).unwrap_or_default().split("\"effects\"").next().unwrap_or_default().to_owned() };
    assert_eq!(only_masks(&mask_before), only_masks(&masks_after), "no mask changed");
}

#[test]
fn rectangles_ellipses_and_polystars_become_bezier_paths_with_the_same_outline() {
    let mut s = comp();
    for (kind, vertices) in [("rect", 4), ("ellipse", 4), ("star", 10), ("polygon", 6)] {
        let layer = s.execute("layer.newShape", json!({"kind": kind})).unwrap()["layer"].as_u64().unwrap();
        let shapes = items(&mut s, layer, "rect").len() + items(&mut s, layer, "ellipse").len() + items(&mut s, layer, "star").len();
        assert_eq!(shapes, 1, "{kind}: one parametric shape to start with");
        let made = s.execute("path.convertToBezier", json!({"layer": layer})).unwrap();
        let uid = made["paths"][0].as_u64().expect("a new Path item");
        let left = items(&mut s, layer, "rect").len() + items(&mut s, layer, "ellipse").len() + items(&mut s, layer, "star").len();
        assert_eq!(left, 0, "{kind}: no parametric shape left");
        let item = items(&mut s, layer, "path").into_iter().find(|item| item["uid"] == uid).expect("the Path item");
        let inner = item["children"].as_array().unwrap().iter().find(|child| child["match"] == "path").unwrap()["uid"].as_u64().unwrap();
        let path = path_of(&mut s, layer, inner);
        assert_eq!(path["vertices"].as_array().map(Vec::len), Some(vertices), "{kind}: {path}");
        assert_eq!(path["closed"], true);
        s.execute("edit.undo", json!({})).unwrap();
        let back = items(&mut s, layer, "rect").len() + items(&mut s, layer, "ellipse").len() + items(&mut s, layer, "star").len();
        assert_eq!(back, 1, "{kind}: one undo brings the shape back");
    }
}

#[test]
fn converting_something_else_is_refused_without_a_change() {
    let mut s = comp();
    let solid = s.execute("layer.newSolid", json!({"name": "S", "width": 10, "height": 10})).unwrap()["layer"].as_u64().unwrap();
    assert!(s.execute("path.convertToBezier", json!({"layer": solid})).is_err());
    let layer = s.execute("layer.newShape", json!({"kind": "rect"})).unwrap()["layer"].as_u64().unwrap();
    let fill = items(&mut s, layer, "fill")[0]["uid"].as_u64().unwrap();
    let revision = s.revision;
    assert!(s.execute("path.convertToBezier", json!({"layer": layer, "shape": fill})).is_err());
    assert!(s.execute("path.convertToBezier", json!({"layer": layer, "shape": 999_999})).is_err());
    assert!(s.execute("path.convertToBezier", json!({"layer": layer, "shape": "x"})).is_err());
    assert_eq!(s.revision, revision);
}
