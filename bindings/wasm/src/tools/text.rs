//! Typing on the canvas: the Text tool's click (a new point-text layer at the click, or the
//! caret in a text layer clicked), a double-click on a text layer with the Selection tool, the
//! caret and selection drawn over the text, and typed text and editing keys through the engine's
//! `text.*` commands. Text typed into one layer in one go is one undo step.

use kition_kit::tools::{ToolMods, ToolOverlay};
use kition_motion_engine::Session;
use kition_motion_engine::commands::text_edit;
use kition_motion_engine::geom::{Mat3, vec2};
use kition_motion_engine::project::{Layer, LayerId, LayerSource};
use kition_motion_engine::render::text::caret_maps;
use kition_motion_engine::text::kurbo::Point;
use kition_motion_engine::text::{TextLayout, layout_doc};
use serde_json::json;

use super::geometry::{self, Pt};
use super::{Gesture, SELECTION, ToolState, with_ctx};
use crate::MotionDocument;

const HIGHLIGHT: [u8; 4] = [62, 136, 255, 90];

fn is_text(layer: &Layer) -> bool {
    matches!(layer.source, LayerSource::Text)
}

/// A text layer's layout and its layer → comp matrix, at the current time.
fn laid_out(session: &Session, id: LayerId) -> Option<(TextLayout, Mat3)> {
    let doc = text_edit::layer_doc(session, id)?;
    let m = with_ctx(session, |ctx| ctx.comp.layer(id).map(|l| ctx.layer_to_comp(l).0)).flatten()?;
    Some((layout_doc(&doc), m))
}

/// The character boundary of layer `id` nearest comp point `p`.
fn char_at(session: &Session, id: LayerId, p: Pt) -> Option<usize> {
    let (layout, m) = laid_out(session, id)?;
    let q = m.inverse()?.apply(vec2(p[0], p[1]));
    (q.x.is_finite() && q.y.is_finite()).then(|| layout.hit(Point::new(q.x, q.y)))
}

/// Whether `p` is on the text being edited (its box).
pub(super) fn inside_edited(session: &Session, p: Pt) -> bool {
    let Some(edit) = &session.state.text_edit else { return false };
    with_ctx(session, |ctx| ctx.comp.layer(edit.layer).and_then(|l| geometry::layer_box(ctx, l)).is_some_and(|b| b.contains(p))).unwrap_or(false)
}

/// The edited text's box (dashed), its selection and its caret.
pub(super) fn overlays(session: &Session, zoom: f64, out: &mut Vec<ToolOverlay>) {
    let Some(edit) = session.state.text_edit.clone() else { return };
    if let Some(b) = with_ctx(session, |ctx| ctx.comp.layer(edit.layer).and_then(|l| geometry::layer_box(ctx, l))).flatten() {
        let c = b.corners();
        let d = format!("M{} {} L{} {} L{} {} L{} {} Z", c[0][0], c[0][1], c[1][0], c[1][1], c[2][0], c[2][1], c[3][0], c[3][1]);
        out.push(ToolOverlay::Path { d, color: SELECTION, width: 1.0, dashed: true });
    }
    let Some((layout, m)) = laid_out(session, edit.layer) else { return };
    // Where animators (and Path Options) put each caret position: the highlight and the caret
    // follow the characters as they are drawn, not their place before animation.
    let maps = with_ctx(session, |ctx| ctx.comp.layer(edit.layer).map(|l| caret_maps(ctx, l, layout.chars))).flatten().unwrap_or_default();
    let to_comp = |p: Point, caret: usize| {
        let moved = maps.get(caret).map_or(vec2(p.x, p.y), |at| at.apply(vec2(p.x, p.y)));
        let q = m.apply(moved);
        [q.x, q.y]
    };
    if !edit.is_empty() {
        // One quad per character, each moved with its character (caret `ci + 1` follows character `ci`).
        let (from, to) = (edit.anchor.min(edit.caret), edit.anchor.max(edit.caret));
        for ci in from..to.min(layout.chars) {
            for quad in layout.selection_quads(ci, ci + 1) {
                out.push(ToolOverlay::Highlight { quad: quad.map(|p| to_comp(p, ci + 1)), color: HIGHLIGHT });
            }
        }
    }
    let (top, bottom) = layout.caret(edit.caret);
    let (a, b) = (to_comp(top, edit.caret), to_comp(bottom, edit.caret));
    out.push(ToolOverlay::Line { ax: a[0], ay: a[1], bx: b[0], by: b[1], color: SELECTION, dashed: false });
    // A second stroke a screen pixel over, so the caret reads at any zoom.
    let nudge = 1.0 / zoom;
    out.push(ToolOverlay::Line { ax: a[0] + nudge, ay: a[1], bx: b[0] + nudge, by: b[1], color: SELECTION, dashed: false });
}

/// Starts typing into layer `id` with `params` for `text.edit`.
fn begin(doc: &mut MotionDocument, st: &mut ToolState, params: serde_json::Value) -> bool {
    match doc.tool_run("text.edit", params) {
        Ok(_) => {
            st.typing = Some(st.merge_key("type"));
            true
        }
        Err(error) => {
            st.status = Some(error);
            false
        }
    }
}

/// The Text tool's press: in the edited text it places the caret; elsewhere while editing it
/// ends the edit; on a text layer it starts editing there; on empty canvas it makes a new text
/// layer whose first line starts at the click.
pub(super) fn text_down(doc: &mut MotionDocument, st: &mut ToolState, p: Pt) {
    if doc.session.state.text_edit.is_some() {
        if inside_edited(&doc.session, p) {
            place_caret(doc, st, p);
        } else {
            doc.end_text(st);
        }
        return;
    }
    let hit = with_ctx(&doc.session, |ctx| geometry::pick(ctx, p, &is_text)).flatten();
    if let Some(id) = hit {
        let caret = char_at(&doc.session, id, p).unwrap_or(0);
        if begin(doc, st, json!({ "layer": id.0, "caret": caret })) {
            st.gesture = Gesture::TextDrag { anchor: caret };
        }
        return;
    }
    match doc.tool_run("layer.newText", json!({ "text": "", "position": p, "justify": "left", "edit": true })) {
        Ok(_) => st.typing = Some(st.merge_key("type")),
        Err(error) => st.status = Some(error),
    }
}

/// A press in the edited text: the caret goes there, and a drag selects from it.
pub(super) fn place_caret(doc: &mut MotionDocument, st: &mut ToolState, p: Pt) {
    let Some(edit) = doc.session.state.text_edit.clone() else { return };
    let Some(at) = char_at(&doc.session, edit.layer, p) else { return };
    let _ = doc.tool_run("text.setSelection", json!({ "start": at }));
    st.gesture = Gesture::TextDrag { anchor: at };
}

/// Dragging over the edited text selects from `anchor` to the pointer.
pub(super) fn drag_select(doc: &mut MotionDocument, anchor: usize, p: Pt) {
    let Some(edit) = doc.session.state.text_edit.clone() else { return };
    let Some(at) = char_at(&doc.session, edit.layer, p) else { return };
    if (edit.anchor, edit.caret) != (anchor, at) {
        let _ = doc.tool_run("text.setSelection", json!({ "anchor": anchor, "caret": at }));
    }
}

/// A double-click with the Selection tool on a text layer edits it, its whole text selected;
/// false when no text layer is under the pointer.
pub(super) fn double_click(doc: &mut MotionDocument, st: &mut ToolState, p: Pt) -> bool {
    let hit = with_ctx(&doc.session, |ctx| geometry::pick(ctx, p, &is_text)).flatten();
    let Some(id) = hit else { return false };
    let n = text_edit::layer_doc(&doc.session, id).map_or(0, |d| d.char_len());
    st.gesture = Gesture::Idle;
    begin(doc, st, json!({ "layer": id.0, "select": [0, n] }));
    true
}

/// Typed text replaces the selection.
pub(super) fn type_text(doc: &mut MotionDocument, st: &mut ToolState, text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let merge = match &st.typing {
        Some(key) => key.clone(),
        None => {
            let key = st.merge_key("type");
            st.typing = Some(key.clone());
            key
        }
    };
    doc.tool_run("text.insert", json!({ "text": text, "merge": merge })).map(|_| ())
}

/// An editing key while typing: Enter a new line, Backspace and Delete, the arrows (Alt by
/// word, Shift extending the selection), Home and End, and Escape to stop.
pub(super) fn key(doc: &mut MotionDocument, st: &mut ToolState, key: &str, mods: ToolMods) {
    let merge = st.typing.clone().unwrap_or_else(|| st.merge_key("type"));
    let moves = |to: &str| json!({ "to": to, "extend": mods.shift });
    let result = match key {
        "Escape" | "Esc" => {
            doc.end_text(st);
            Ok(serde_json::Value::Null)
        }
        "Enter" | "Return" => doc.tool_run("text.insert", json!({ "text": "\n", "merge": merge })),
        "Backspace" => doc.tool_run("text.delete", json!({ "direction": "backward", "word": mods.alt, "merge": merge })),
        "Delete" => doc.tool_run("text.delete", json!({ "direction": "forward", "word": mods.alt, "merge": merge })),
        "ArrowLeft" | "Left" => doc.tool_run("text.moveCaret", moves(if mods.alt { "wordLeft" } else { "left" })),
        "ArrowRight" | "Right" => doc.tool_run("text.moveCaret", moves(if mods.alt { "wordRight" } else { "right" })),
        "ArrowUp" | "Up" => doc.tool_run("text.moveCaret", moves("up")),
        "ArrowDown" | "Down" => doc.tool_run("text.moveCaret", moves("down")),
        "Home" => doc.tool_run("text.moveCaret", moves("lineStart")),
        "End" => doc.tool_run("text.moveCaret", moves("lineEnd")),
        "a" | "A" if mods.command => doc.tool_run("text.setSelection", json!({ "select": "all" })),
        _ => Ok(serde_json::Value::Null),
    };
    if let Err(error) = result {
        st.status = Some(error);
    }
}
