//! A command refuses a parameter its schema does not name, saying which, instead of running
//! without it (engine rule E7a, kit requirement AX15).
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::json;

fn project() -> MotionDocument {
    MotionDocument::create("Project.kmotion", 64, 36).unwrap()
}

#[test]
fn every_command_refuses_a_parameter_it_does_not_take() {
    if let Err(problems) = kition_kit::testing::check_unknown_params(project, &[]) {
        panic!("{} commands:\n{}", problems.len(), problems.join("\n"));
    }
}

#[test]
fn a_misspelled_parameter_names_the_one_it_resembles_and_changes_nothing() {
    let mut doc = project();
    let before = doc.save(None).unwrap();
    let error = doc.execute("layer.newSolid", json!({ "name": "Red", "colour": "#ff0000" })).unwrap_err();
    assert!(error.contains("`colour`") && error.contains("did you mean `color`"), "{error}");
    assert_eq!(doc.save(None).unwrap(), before);
}
