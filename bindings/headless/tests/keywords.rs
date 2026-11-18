//! `effect.apply` carries every effect's name as its keywords (KHP command keywords), so an
//! agent's command search for an effect (`gradient ramp`, `gaussian blur`) finds the command the
//! effect is a parameter of. khp-bench's agents searched `ramp` and `blur` and found none.

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;

#[test]
fn effect_apply_is_found_by_the_effects_names() {
    let doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    let commands = doc.commands();
    let apply = commands.iter().find(|command| command.id == "effect.apply").expect("effect.apply is listed");
    for name in ["Gradient Ramp", "Gaussian Blur", "Glow"] {
        assert!(apply.keywords.iter().any(|keyword| keyword == name), "{name} is a keyword of effect.apply");
    }
    assert!(
        commands.iter().filter(|command| command.id != "effect.apply").all(|command| !command.keywords.iter().any(|keyword| apply.keywords.contains(keyword))),
        "only effect.apply has the effect names"
    );
}

/// khp-bench r2: an agent asked for `layer.delete`, which does not exist; `edit.clear` deletes
/// layers, and its keywords say so, so a search for "delete layer" and the unknown id find it.
#[test]
fn edit_clear_is_found_as_the_command_that_deletes_layers() {
    let doc = MotionDocument::create("title.kmotion", 64, 36).unwrap();
    let commands = doc.commands();
    let clear = commands.iter().find(|command| command.id == "edit.clear").expect("edit.clear is listed");
    assert!(clear.keywords.iter().any(|keyword| keyword.contains("delete") && keyword.contains("layer")), "{:?}", clear.keywords);
    assert!(clear.description.contains("deletes layers"), "{}", clear.description);
}
