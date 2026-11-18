//! Commands that serialize or replace the complete project consume its complete contents.
use kition_kit::{Premise, Under, Within};
use kition_motion_engine::Session;
use serde_json::{Value, json};

fn whole(command: &str, params: &Value) -> bool {
    matches!(
        command,
        "file.newProject"
            | "file.openDemoProject"
            | "file.open"
            | "file.save"
            | "file.saveAs"
            | "file.incrementAndSave"
            | "file.revert"
            | "file.closeProject"
            | "file.saveCopy"
            | "file.collectFiles"
            | "file.openRecent"
            | "templates.create"
            | "templates.saveAs"
    ) || command == "edit.history.goto" && params.get("steps").is_none()
}
pub(crate) fn check(session: &Session, command: &str, params: &Value) -> Result<(), String> {
    if !whole(command, params) {
        return Ok(());
    }
    let mut objects = session.project.items.len().saturating_add(session.project.render_queue.len());
    for item in session.project.items.values() {
        if let Some(comp) = item.as_comp() {
            objects = objects.saturating_add(comp.layers.len());
        }
    }
    if objects > 65_536 {
        return Err("too many project objects to guard a complete-project operation; split or simplify the project".to_owned());
    }
    Ok(())
}
pub(crate) fn extend(session: &Session, command: &str, params: &Value, under: &mut Option<Under>) -> Result<(), String> {
    if !whole(command, params) {
        return Ok(());
    }
    check(session, command, params)?;
    let answer = under.get_or_insert_with(Under::default);
    answer.premises.push(Premise::on("project").fields(["attrs"]).positions(Vec::<String>::new()));
    for (field, len) in [("items", session.project.items.len()), ("renderQueue", session.project.render_queue.len())] {
        answer.within.push(Within { parent: json!("project"), field: field.to_owned(), from: 0, to: len.saturating_add(1), ..Default::default() });
    }
    for item in session.project.items.values() {
        answer.premises.push(Premise::on(json!(item.id.0)));
        if let Some(comp) = item.as_comp() {
            for layer in &comp.layers {
                answer.premises.push(Premise::on(json!(layer.id.0)));
            }
        }
    }
    for item in &session.project.render_queue {
        answer.premises.push(Premise::on(format!("render:{}", item.id)));
    }
    Ok(())
}

// Replacing the engine project resets its saved revision. The kit still owns the old host
// file, so the replacement must be saved through that host before it is considered clean.
pub(crate) fn after(session: &mut Session, command: &str) {
    if matches!(command, "file.newProject" | "file.openDemoProject" | "file.closeProject") {
        session.mark_unsaved();
    }
}
