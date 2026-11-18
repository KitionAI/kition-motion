//! The guide's first example runs exactly as written and leaves nothing a designer would point
//! out (video plan P0): its old example was a 320 × 180 comp with 40 px text on black, the flat,
//! plain look judges marked down, and models copy the guide's examples.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;
use serde_json::Value;

const GUIDE: &str = include_str!("../../../agent/guide.md");

/// The `motion_run` calls in the guide section that starts with `heading`, in order.
fn runs(heading: &str) -> Vec<Value> {
    let start = GUIDE.find(heading).unwrap_or_else(|| panic!("no section {heading}"));
    let section = &GUIDE[start..];
    let section = &section[..section[3..].find("\n## ").map_or(section.len(), |n| n + 3)];
    let mut out = vec![];
    let mut rest = section;
    while let Some(at) = rest.find("motion_run {") {
        let json = &rest[at + "motion_run ".len()..];
        let mut values = serde_json::Deserializer::from_str(json).into_iter::<Value>();
        out.push(values.next().unwrap().unwrap_or_else(|e| panic!("the guide's motion_run is not JSON: {e}")));
        rest = &json[values.byte_offset()..];
    }
    out
}

#[test]
fn the_guides_first_example_runs_as_written_and_leaves_no_design_advice() {
    let calls = runs("## Example 1");
    assert_eq!(calls.len(), 2);
    let mut doc = MotionDocument::create("Title.kmotion", 1920, 1080).unwrap();
    let mut made = vec![];
    for call in &calls {
        for step in call["steps"].as_array().unwrap() {
            let (command, params) = (step["command"].as_str().unwrap(), step["params"].clone());
            let answer = doc.execute(command, params.clone()).unwrap_or_else(|e| panic!("{command} {params}: {e}"));
            made.extend(answer["layer"].as_u64());
        }
    }
    // The ids the guide's answer gives, which its second call uses.
    assert_eq!(made, [4, 27, 66]);
    let advice = &doc.inspect().unwrap()["design"];
    assert_eq!(advice, &Value::Array(vec![]), "{advice}");
    // motion_check, which agents call before they export, reports the same advice: none here.
    assert_eq!(doc.check(), vec![]);
}
