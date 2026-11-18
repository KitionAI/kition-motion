#![allow(clippy::unwrap_used, clippy::panic)]

use kition_motion_engine::Session;
use kition_motion_project::ItemId;
use serde_json::json;

#[test]
fn replacement_solid_preserves_each_explicit_dimension() {
    for (parameters, dimensions) in
        [(json!({"height":100}), (64, 100)), (json!({"width":80}), (80, 32)), (json!({"width":80,"height":100}), (80, 100)), (json!({}), (64, 32))]
    {
        let mut session = Session::default();
        session.execute("comp.new", json!({"width":320,"height":180})).unwrap();
        let item = session.execute("file.importPlaceholder", json!({"width":64,"height":32})).unwrap()["item"].as_u64().unwrap();
        let mut parameters = parameters;
        parameters["item"] = json!(item);
        session.execute("file.replaceWithSolid", parameters.clone()).unwrap();
        assert_eq!(session.project.item(ItemId(item)).unwrap().dimensions(), Some(dimensions), "{parameters}");
    }
}
