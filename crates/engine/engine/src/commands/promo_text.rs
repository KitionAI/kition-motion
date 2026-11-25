//! Promo text presets for `layer.applyTextPreset`: per-unit timed entrances (an Expression
//! Selector gives every word or character its own start, duration and cubic-bezier ease, as
//! motion designers do), a fast typewriter and a count-up (Source Text
//! expressions), and an odometer digit roll (one masked strip layer per digit, parented to the
//! text). Chinese and other CJK text animates per character.
//!
//! Every preset is one undo step; timing parameters are frames of the comp, `start` is comp time
//! (default the current time).

use kition_motion_keyframe::{Justify, TextDoc, Value as KV};
use kition_motion_project::build::{self, Ids};
use kition_motion_project::{Comp, Expression, Layer, LayerId, LayerSource, MatteKind, PropGroup, TrackMatte};
use kition_motion_text::TextLayout;
use kition_motion_time::Tick;
use serde_json::{Value, json};

use super::promo_ease::{self as pe, Curve};
use super::{bad, f_p, layer_mut, layer_p, str_p};
use crate::{EngineError, Result, Session};

const C: &str = "layer.applyTextPreset";

/// (id, name, description) of the promo presets.
pub(crate) const PRESETS: &[(&str, &str, &str)] = &[
    (
        "blurSlide",
        "Blur Slide",
        "Words rise 64 px out of an 18 px blur and fade in, one ease-out each (22 frames, 4 frames apart); variant subtitle: 26 px, 9 px, 20 frames, 2.4 apart. CJK text goes per character.",
    ),
    ("letterpress", "Letterpress", "Characters press in from 1.6x scale and a 6 px blur while fading in, 12 frames each, 3 frames apart."),
    (
        "leadWordZoom",
        "Lead Word Zoom",
        "The first word holds centred at 2.3x and keeps pushing 6%, then one curve shrinks it into the line as the line slides into place and the other words push in 0.5 em, 4 frames apart.",
    ),
    ("typewriterFast", "Typewriter Fast", "Types 0.7 frames per character with a cursor that blinks once typing ends (Source Text expression)."),
    (
        "digitRoll",
        "Digit Roll",
        "Every digit rolls into place like an odometer (a 0-9 strip layer per digit under a window matte, parented to the text), 22 frames each, 4 frames apart, tabular figures.",
    ),
    (
        "countUp",
        "Count Up",
        "The number in the text counts from `from` to `to` (default the number written) with an expo-out ease and thousands separators (Source Text expression).",
    ),
];

pub(crate) fn is_promo(id: &str) -> Option<&'static str> {
    PRESETS.iter().find(|(p, n, _)| p.eq_ignore_ascii_case(id) || n.eq_ignore_ascii_case(id)).map(|(p, ..)| *p)
}

/// A non-negative finite number parameter.
fn num(p: &Value, k: &str, d: f64) -> Result<f64> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(d),
        Some(v) => v.as_f64().filter(|x| x.is_finite() && *x >= 0.0).ok_or_else(|| bad(C, format!("`{k}` must be a number ≥ 0"))),
    }
}

fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xAC00..=0xD7AF | 0x20000..=0x2FA1F))
}

/// Based On for a per-unit preset: `by`, else words for spaced Latin text and characters
/// (excluding spaces) for CJK.
fn based_on(p: &Value, text: &str, words: bool) -> Result<u32> {
    Ok(match str_p(p, "by") {
        Some("words") => 2,
        Some("characters" | "chars") => 1,
        Some("lines") => 3,
        Some(x) => return Err(bad(C, format!("`by` must be words, characters or lines, not {x}"))),
        None if words && !has_cjk(text) => 2,
        None => 1,
    })
}

pub(crate) fn source_doc(l: &Layer, t: Tick) -> Option<TextDoc> {
    match l.props.prop("text/sourceText").map(|pr| pr.value_at(t)) {
        Some(KV::Text(d)) => Some(*d),
        _ => None,
    }
}

fn per_char_3d(l: &Layer) -> bool {
    l.props.prop("text/perChar3d").is_some_and(|p| p.value.as_bool())
}

fn js_num(x: f64) -> String {
    if x.is_finite() { format!("{x:.6}") } else { "0".into() }
}

fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// Expression Selector amount: unit `textIndex` (from `first`) starts `stagger` s after the one
/// before it at `t0`, runs `dur` s through `curve`; 100 % (the animator's values) before it
/// starts, 0 % once it lands. Units before `first` are not affected.
fn stagger_expr(curve: &Curve, t0: f64, first: u32, stagger: f64, dur: f64) -> String {
    format!(
        "{}\nvar i = textIndex - {first};\nif (i < 0) {{ 0; }} else {{ var t = (time - {} - i * {}) / {}; 100 * (1 - promoEase(t < 0 ? 0 : (t > 1 ? 1 : t))); }}",
        curve.js("promoEase"),
        js_num(t0),
        js_num(stagger),
        js_num(dur.max(1e-6)),
    )
}

fn expression_selector(ids: &mut Ids, based: u32, expr: String) -> PropGroup {
    let mut g = build::expression_selector(ids, "Expression Selector 1");
    if let Some(b) = g.prop_mut("basedOn") {
        b.value = KV::Enum(based);
    }
    if let Some(a) = g.prop_mut("amount") {
        a.expr = Some(Expression { text: expr, enabled: true });
    }
    g
}

/// A text animator with `props` (kind, components) and `selector` replacing its Range Selector
/// (`None` keeps the Range Selector: every character, fully).
fn animator(ids: &mut Ids, name: &str, three: bool, props: &[(&str, &[f64])], selector: Option<PropGroup>) -> PropGroup {
    let mut g = build::text_animator(ids, name, vec![]);
    if let Some(sel) = selector
        && let Some(s) = g.sub_mut("selectors")
    {
        s.children.clear();
        s.children.push(sel.into());
    }
    for (kind, comps) in props {
        super::text_anim::add_props(ids, &mut g, kind, three);
        if let Some(pr) = g.sub_mut("properties").and_then(|pg| pg.get_mut(kind)) {
            pr.value = pr.value.with_components(comps);
        }
    }
    g
}

fn anim_prop<'a>(g: &'a mut PropGroup, kind: &str) -> Option<&'a mut kition_motion_project::Property> {
    g.sub_mut("properties").and_then(|pg| pg.get_mut(kind))
}

/// Set More Options: anchor point grouping and grouping alignment (percent).
fn grouping(l: &mut Layer, mode: u32, align: [f64; 2]) {
    if let Some(m) = l.props.prop_mut("text/moreOptions/anchorGrouping") {
        m.value = KV::Enum(mode);
    }
    if let Some(m) = l.props.prop_mut("text/moreOptions/groupingAlignment")
        && m.keys.is_empty()
    {
        m.value = KV::Vec2(align);
    }
}

fn push_animator(l: &mut Layer, g: PropGroup) -> Result<u64> {
    let anims = l.props.group_mut("text/animators").ok_or_else(|| bad(C, "not a text layer"))?;
    let uid = g.uid;
    anims.children.push(g.into());
    Ok(uid)
}

/// What every promo preset reads before editing.
struct Ctx {
    cid: kition_motion_project::ItemId,
    lid: LayerId,
    comp: Comp,
    layer: Layer,
    doc: TextDoc,
    lay: TextLayout,
    start: Tick,
    fps: f64,
}

impl Ctx {
    fn new(s: &Session, p: &Value) -> Result<Ctx> {
        let (cid, lid) = layer_p(s, p, C)?;
        let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?.clone();
        let layer = comp.layer(lid).cloned().ok_or_else(|| bad(C, "no layer"))?;
        if !matches!(layer.source, LayerSource::Text) {
            return Err(bad(C, "not a text layer"));
        }
        let start = match p.get("start") {
            None | Some(Value::Null) => s.time_of(cid),
            Some(v) => Tick::from_seconds_f64(v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| bad(C, "`start` must be comp seconds"))?),
        };
        let doc = source_doc(&layer, layer.layer_time(start)).ok_or_else(|| bad(C, "not a text layer"))?;
        let lay = kition_motion_text::layout_doc(&doc);
        let fps = pe::fps(&comp);
        Ok(Ctx { cid, lid, comp, layer, doc, lay, start, fps })
    }
    fn secs(&self, frames: f64) -> f64 {
        frames / self.fps
    }
    /// Comp time `start + frames` as layer time.
    fn key_t(&self, frames: f64) -> Tick {
        self.layer.layer_time(self.start + pe::frames(&self.comp, frames))
    }
}

pub(crate) fn apply(s: &mut Session, p: &Value, id: &str) -> Result<Value> {
    let x = Ctx::new(s, p)?;
    let name = PRESETS.iter().find(|(i, ..)| *i == id).map(|(_, n, _)| *n).unwrap_or(id);
    let label = format!("Apply Text Preset {name}");
    match id {
        "blurSlide" => blur_slide(s, p, x, &label),
        "letterpress" => letterpress(s, p, x, &label),
        "leadWordZoom" => lead_word_zoom(s, p, x, &label),
        "typewriterFast" => typewriter(s, p, x, &label),
        "digitRoll" => digit_roll(s, p, x, &label),
        "countUp" => count_up(s, p, x, &label),
        _ => Err(bad(C, format!("no preset {id}"))),
    }
}

fn blur_slide(s: &mut Session, p: &Value, x: Ctx, label: &str) -> Result<Value> {
    let subtitle = match str_p(p, "variant") {
        None | Some("default") => false,
        Some("subtitle") => true,
        Some(v) => return Err(bad(C, format!("`variant` must be default or subtitle, not {v}"))),
    };
    let (dy0, blur0, fr0, st0) = if subtitle { (26.0, 9.0, 20.0, 2.4) } else { (64.0, 18.0, 22.0, 4.0) };
    let dy = p.get("dy").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(dy0);
    let (blur, frames, stagger) = (num(p, "blur", blur0)?, num(p, "frames", fr0)?, num(p, "stagger", st0)?);
    let curve = pe::curve_p(p, "ease", Curve::new(0.22, 1.0, 0.36, 1.0), C)?;
    let based = based_on(p, &x.doc.text, true)?;
    let expr = stagger_expr(&curve, x.start.seconds(), 1, x.secs(stagger), x.secs(frames));
    let (cid, lid) = (x.cid, x.lid);
    let mb = pe::motion_blur_p(p);
    let uid = s.edit(label, None, |proj, _| {
        let mut next = proj.next_id;
        let mut ids = Ids(&mut next);
        let l = layer_mut(proj, cid, lid)?;
        let three = per_char_3d(l);
        let sel = expression_selector(&mut ids, based, expr.clone());
        // Text animator Blur is a blurriness (twice the Gaussian sigma); `blur` is a CSS radius.
        let g = animator(&mut ids, "Blur Slide", three, &[("position", &[0.0, dy, 0.0]), ("blur", &[blur * 2.0, blur * 2.0]), ("opacity", &[0.0])], Some(sel));
        let uid = push_animator(l, g)?;
        proj.next_id = next;
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &[lid], pe::SHUTTER);
        }
        Ok(uid)
    })?;
    Ok(
        json!({"preset": "blurSlide", "animators": [uid], "units": unit_name(based), "end": x.start.seconds() + x.secs(frames + stagger * units(&x.lay, based).saturating_sub(1) as f64)}),
    )
}

fn unit_name(based: u32) -> &'static str {
    match based {
        2 => "words",
        3 => "lines",
        _ => "characters",
    }
}

fn units(lay: &TextLayout, based: u32) -> usize {
    match based {
        2 => lay.words,
        3 => lay.lines,
        1 => lay.chars_no_space,
        _ => lay.chars,
    }
}

fn letterpress(s: &mut Session, p: &Value, x: Ctx, label: &str) -> Result<Value> {
    let scale = num(p, "scale", 1.6)?;
    let (blur, frames, stagger) = (num(p, "blur", 6.0)?, num(p, "frames", 12.0)?, num(p, "stagger", 3.0)?);
    let curve = pe::curve_p(p, "ease", Curve::new(0.2, 0.7, 0.25, 1.0), C)?;
    let based = based_on(p, &x.doc.text, false)?;
    let expr = stagger_expr(&curve, x.start.seconds(), 1, x.secs(stagger), x.secs(frames));
    let (cid, lid) = (x.cid, x.lid);
    let mb = pe::motion_blur_p(p);
    let uid = s.edit(label, None, |proj, _| {
        let mut next = proj.next_id;
        let mut ids = Ids(&mut next);
        let l = layer_mut(proj, cid, lid)?;
        let three = per_char_3d(l);
        // Characters scale about their visual centre, not their baseline.
        grouping(l, 0, [0.0, -35.0]);
        let sel = expression_selector(&mut ids, based, expr.clone());
        let sc = scale * 100.0;
        let g = animator(&mut ids, "Letterpress", three, &[("scale", &[sc, sc, 100.0]), ("blur", &[blur * 2.0, blur * 2.0]), ("opacity", &[0.0])], Some(sel));
        let uid = push_animator(l, g)?;
        proj.next_id = next;
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &[lid], pe::SHUTTER);
        }
        Ok(uid)
    })?;
    Ok(
        json!({"preset": "letterpress", "animators": [uid], "units": unit_name(based), "end": x.start.seconds() + x.secs(frames + stagger * units(&x.lay, based).saturating_sub(1) as f64)}),
    )
}

fn lead_word_zoom(s: &mut Session, p: &Value, x: Ctx, label: &str) -> Result<Value> {
    let scale = num(p, "scale", 2.3)?;
    let push = num(p, "push", 0.06)?;
    let hold = num(p, "hold", 30.0)?;
    let shrink = num(p, "frames", 24.0)?;
    let stagger = num(p, "stagger", 4.0)?;
    let follow = 12.0;
    let curve = pe::curve_p(p, "ease", Curve::new(0.5, 0.0, 0.05, 1.0), C)?;
    let follow_curve = Curve::new(0.22, 0.8, 0.36, 1.0);
    // Words, unless the text is one word (CJK without spaces): then its first character leads.
    let based = match str_p(p, "by") {
        Some(_) => based_on(p, &x.doc.text, true)?,
        None if x.lay.words > 1 => 2,
        None => 1,
    };
    let lead: Vec<&kition_motion_text::CharGlyph> = x
        .lay
        .glyphs
        .iter()
        .filter(|g| !g.is_space && g.line_index == 0)
        .filter(|g| match based {
            2 => g.word_index == 0,
            3 => true,
            _ => g.char_index_no_space == 0,
        })
        .collect();
    let (Some(lx0), Some(lx1)) = (lead.iter().map(|g| g.origin.x).reduce(f64::min), lead.iter().map(|g| g.origin.x + g.advance).reduce(f64::max)) else {
        return Err(bad(C, "the text is empty"));
    };
    let line = x.lay.line_boxes.first().copied().unwrap_or([0.0; 4]);
    let dx = line[0] + line[2] / 2.0 - (lx0 + lx1) / 2.0;
    let em = x.doc.size;
    let t1 = hold;
    let t2 = hold + shrink;
    let follow_expr = stagger_expr(&follow_curve, x.start.seconds() + x.secs(t1), 2, x.secs(stagger), x.secs(follow));
    let lead_expr = "textIndex == 1 ? 100 : 0".to_string();
    let (k0, k1, k2) = (x.key_t(0.0), x.key_t(t1), x.key_t(t2));
    let (cid, lid) = (x.cid, x.lid);
    let mb = pe::motion_blur_p(p);
    let s0 = scale * 100.0;
    let s1 = s0 * (1.0 + push);
    let uids = s.edit(label, None, |proj, _| {
        let mut next = proj.next_id;
        let mut ids = Ids(&mut next);
        let l = layer_mut(proj, cid, lid)?;
        let three = per_char_3d(l);
        grouping(l, if based == 2 { 1 } else { 0 }, [0.0, -35.0]);
        let mut shift = animator(&mut ids, "Lead Line Shift", three, &[("position", &[dx, 0.0, 0.0])], None);
        if let Some(pr) = anim_prop(&mut shift, "position") {
            let v = |d: f64| pr.value.with_components(&[d, 0.0, 0.0]);
            let keys = [(k0, v(dx)), (k1, v(dx)), (k2, v(0.0))];
            pe::set_keys(pr, &keys, &[Curve::LINEAR, curve]);
        }
        let sel = expression_selector(&mut ids, based, lead_expr.clone());
        let mut zoom = animator(&mut ids, "Lead Word", three, &[("scale", &[s0, s0, 100.0])], Some(sel));
        if let Some(pr) = anim_prop(&mut zoom, "scale") {
            let v = |d: f64| pr.value.with_components(&[d, d, 100.0]);
            let keys = [(k0, v(s0)), (k1, v(s1)), (k2, v(100.0))];
            pe::set_keys(pr, &keys, &[Curve::LINEAR, curve]);
        }
        let sel = expression_selector(&mut ids, based, follow_expr.clone());
        let rest = animator(&mut ids, "Lead Follow", three, &[("position", &[0.5 * em, 0.0, 0.0]), ("opacity", &[0.0])], Some(sel));
        let uids = vec![push_animator(l, shift)?, push_animator(l, zoom)?, push_animator(l, rest)?];
        proj.next_id = next;
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &[lid], pe::SHUTTER);
        }
        Ok(uids)
    })?;
    Ok(
        json!({"preset": "leadWordZoom", "animators": uids, "units": unit_name(based), "end": x.start.seconds() + x.secs(t2.max(t1 + follow + stagger * units(&x.lay, based).saturating_sub(2) as f64))}),
    )
}

/// Make a one-line centred or right-aligned text left-aligned without moving it (the anchor point
/// follows), so text that grows from its start does not re-centre while it grows.
fn left_align_in_place(l: &mut Layer, doc: &TextDoc, lay: &TextLayout) {
    if doc.justify == Justify::Left || lay.lines != 1 || doc.box_size.is_some() {
        return;
    }
    let mut left = doc.clone();
    left.justify = Justify::Left;
    let ll = kition_motion_text::layout_doc(&left);
    let (Some(a), Some(b)) = (lay.glyphs.first(), ll.glyphs.first()) else { return };
    let dx = b.origin.x - a.origin.x;
    if let Some(pr) = l.props.prop_mut("text/sourceText") {
        if let KV::Text(d) = &mut pr.value {
            d.justify = Justify::Left;
        }
        for k in &mut pr.keys {
            if let KV::Text(d) = &mut k.value {
                d.justify = Justify::Left;
            }
        }
    }
    if let Some(pr) = l.props.prop_mut("transform/anchor") {
        let shift = |v: &KV| {
            let mut c = v.components();
            if let Some(x0) = c.first_mut() {
                *x0 += dx;
            }
            v.with_components(&c)
        };
        pr.value = shift(&pr.value);
        for k in &mut pr.keys {
            k.value = shift(&k.value);
        }
    }
}

fn typewriter(s: &mut Session, p: &Value, x: Ctx, label: &str) -> Result<Value> {
    let per = num(p, "frames", 0.7)?.max(0.01);
    let cursor = match p.get("cursor") {
        None | Some(Value::Null) => "|".to_string(),
        Some(Value::String(c)) => c.clone(),
        Some(_) => return Err(bad(C, "`cursor` must be a string (\"\" for none)")),
    };
    let cpf = x.secs(per);
    let expr = format!(
        "// Typewriter Fast: {per} frames per character.\n\
var a = Array.from(\"\" + value), cpf = {}, t = time - {};\n\
var n = t < 0 ? 0 : Math.min(a.length, Math.floor(t / cpf) + 1);\n\
var on = n < a.length ? true : (Math.floor((t - a.length * cpf) / 0.5) % 2 == 0);\n\
a.slice(0, n).join(\"\") + (on ? {} : \"\");",
        js_num(cpf),
        js_num(x.start.seconds()),
        js_str(&cursor)
    );
    let n = x.doc.text.chars().count();
    let (cid, lid, doc, lay) = (x.cid, x.lid, x.doc.clone(), x.lay.clone());
    s.edit(label, None, |proj, _| {
        let l = layer_mut(proj, cid, lid)?;
        left_align_in_place(l, &doc, &lay);
        let pr = l.props.prop_mut("text/sourceText").ok_or_else(|| bad(C, "not a text layer"))?;
        pr.expr = Some(Expression { text: expr.clone(), enabled: true });
        Ok(())
    })?;
    Ok(json!({"preset": "typewriterFast", "expression": "text/sourceText", "end": x.start.seconds() + cpf * n as f64}))
}

/// The first number written in `text`: (byte range, value, decimals).
fn find_number(text: &str) -> Option<(std::ops::Range<usize>, f64, u32)> {
    let start = text.char_indices().find(|(_, c)| c.is_ascii_digit())?.0;
    let mut end = start;
    let mut seen_dot = false;
    for (i, c) in text[start..].char_indices() {
        let ok = c.is_ascii_digit() || (c == ',' && !seen_dot) || (c == '.' && !seen_dot);
        if !ok {
            break;
        }
        if c == '.' {
            seen_dot = true;
        }
        end = start + i + c.len_utf8();
    }
    let raw = text.get(start..end)?.trim_end_matches([',', '.']);
    let end = start + raw.len();
    let clean: String = raw.chars().filter(|c| *c != ',').collect();
    let decimals = clean.split_once('.').map_or(0, |(_, d)| d.len() as u32);
    Some((start..end, clean.parse().ok()?, decimals))
}

fn count_up(s: &mut Session, p: &Value, x: Ctx, label: &str) -> Result<Value> {
    let text = x.doc.text.clone();
    let found = find_number(&text);
    let (prefix, suffix, written, dec0) = match &found {
        Some((r, v, d)) => (text.get(..r.start).unwrap_or(""), text.get(r.end..).unwrap_or(""), Some(*v), *d),
        None => (text.as_str(), "", None, 0),
    };
    let to = match f_p(p, "to") {
        Some(v) if v.is_finite() => v,
        Some(_) => return Err(bad(C, "`to` must be a number")),
        None => written.ok_or_else(|| bad(C, "the text has no number; give `to`"))?,
    };
    let from = p.get("from").and_then(Value::as_f64).filter(|v| v.is_finite()).unwrap_or(0.0);
    let duration = num(p, "duration", 1.2)?.max(1e-3);
    let decimals = p.get("decimals").and_then(Value::as_u64).map(|d| d.min(6) as u32).unwrap_or(dec0);
    let sep = match p.get("separator") {
        None | Some(Value::Null) => ",".to_string(),
        Some(Value::String(c)) => c.clone(),
        Some(_) => return Err(bad(C, "`separator` must be a string (\"\" for none)")),
    };
    let expr = format!(
        "// Count Up: {from} to {to} over {duration} s, expo-out.\n\
var x = (time - {}) / {};\n\
x = x < 0 ? 0 : (x > 1 ? 1 : x);\n\
var e = x >= 1 ? 1 : 1 - Math.pow(2, -10 * x);\n\
var v = {} + ({} - {}) * e, neg = v < 0;\n\
var parts = Math.abs(v).toFixed({decimals}).split(\".\"), ip = parts[0], out = \"\";\n\
while (ip.length > 3) {{ out = {} + ip.slice(ip.length - 3) + out; ip = ip.slice(0, ip.length - 3); }}\n\
out = ip + out + (parts.length > 1 ? \".\" + parts[1] : \"\");\n\
{} + (neg ? \"-\" : \"\") + out + {};",
        js_num(x.start.seconds()),
        js_num(duration),
        js_num(from),
        js_num(to),
        js_num(from),
        js_str(&sep),
        js_str(prefix),
        js_str(suffix)
    );
    let (cid, lid) = (x.cid, x.lid);
    s.edit(label, None, |proj, _| {
        let l = layer_mut(proj, cid, lid)?;
        let pr = l.props.prop_mut("text/sourceText").ok_or_else(|| bad(C, "not a text layer"))?;
        if let KV::Text(d) = &mut pr.value {
            d.set_attr("figureWidth", &json!("tabular"), None).map_err(|e| bad(C, e))?;
        }
        pr.expr = Some(Expression { text: expr.clone(), enabled: true });
        Ok(())
    })?;
    Ok(json!({"preset": "countUp", "expression": "text/sourceText", "from": from, "to": to, "end": x.start.seconds() + duration}))
}

fn digit_roll(s: &mut Session, p: &Value, x: Ctx, label: &str) -> Result<Value> {
    let (frames, stagger) = (num(p, "frames", 22.0)?, num(p, "stagger", 4.0)?);
    let cycles = p.get("cycles").and_then(Value::as_u64).unwrap_or(1).min(5) as usize;
    let curve = pe::curve_p(p, "ease", Curve::new(0.25, 0.8, 0.25, 1.0), C)?;
    // Tabular figures: every digit cell is as wide as every other.
    let mut doc = x.doc.clone();
    doc.set_attr("figureWidth", &json!("tabular"), None).map_err(|e| bad(C, e))?;
    let lay = kition_motion_text::layout_doc(&doc);
    let digits: Vec<(usize, kition_motion_text::CharGlyph)> =
        lay.glyphs.iter().filter(|g| g.ch.is_ascii_digit()).enumerate().map(|(i, g)| (i, g.clone())).collect();
    if digits.is_empty() {
        return Err(bad(C, "the text has no digits to roll"));
    }
    let size = doc.size.max(1.0);
    let lead = doc.leading.unwrap_or(size * 1.2).max(size * 0.8);
    let rows = 10 * (cycles + 1);
    let strip_text: String = (0..rows).map(|i| char::from(b'0' + (i % 10) as u8).to_string()).collect::<Vec<_>>().join("\n");
    let mut strip = doc.clone();
    strip.set_text(&strip_text);
    strip.justify = Justify::Left;
    strip.box_size = None;
    strip.leading = Some(lead);
    strip.set_attr("leading", &json!(lead), None).map_err(|e| bad(C, e))?;
    let slay = kition_motion_text::layout_doc(&strip);
    let so = slay.glyphs.first().map(|g| [g.origin.x, g.origin.y]).unwrap_or([0.0, 0.0]);
    // The pitch between strip rows as laid out.
    let pitch = match (slay.glyphs.first(), slay.glyphs.iter().find(|g| g.line_index == 1)) {
        (Some(a), Some(b)) => (b.origin.y - a.origin.y).abs().max(1.0),
        _ => lead,
    };
    let hide: Vec<String> = digits.iter().map(|(_, g)| (g.char_index + 1).to_string()).collect();
    let hide_expr = format!("[{}].indexOf(textIndex) >= 0 ? 100 : 0", hide.join(", "));
    let comp = x.comp.clone();
    let names: Vec<String> = (0..digits.len()).map(|i| comp.unique_layer_name(&format!("{} Digit {}", x.layer.name, i + 1))).collect();
    let (cid, lid) = (x.cid, x.lid);
    let mb = pe::motion_blur_p(p);
    let keys: Vec<(Tick, Tick)> = digits.iter().map(|(i, _)| (x.key_t(*i as f64 * stagger), x.key_t(*i as f64 * stagger + frames))).collect();
    let matte_names: Vec<String> = names.iter().map(|n| format!("{n} Window")).collect();
    let made = s.edit(label, None, |proj, st| {
        let mut made = vec![];
        let mut strips: Vec<LayerId> = vec![];
        let orig = proj.comp(cid).and_then(|c| c.layer(lid)).cloned().ok_or(EngineError::NoComp)?;
        let at = proj.comp(cid).and_then(|c| c.layers.iter().position(|l| l.id == lid)).unwrap_or(0);
        for (n, ((_, g), (ka, kb))) in digits.iter().zip(&keys).enumerate() {
            let d = g.ch.to_digit(10).unwrap_or(0) as f64;
            let timing = |l: &mut Layer| {
                l.start_time = orig.start_time;
                l.in_point = orig.in_point;
                l.out_point = orig.out_point;
                l.stretch = orig.stretch;
                l.switches.three_d = orig.switches.three_d;
                l.parent = Some(lid);
            };
            // The window: a shape one row tall around the digit's middle, a little wider than its
            // cell, used as the strip's alpha matte (hidden itself).
            let mut matte = build::layer(
                proj,
                &comp,
                matte_names.get(n).map(String::as_str).unwrap_or("Digit Window"),
                LayerSource::Shape,
                (comp.width, comp.height),
                None,
            );
            timing(&mut matte);
            matte.switches.video = false;
            if let Some(pr) = matte.props.prop_mut("transform/position") {
                pr.value = KV::Vec3([g.origin.x, g.origin.y, 0.0]);
            }
            let mut next = proj.next_id;
            let mut ids = Ids(&mut next);
            let rect = build::shape_rect(&mut ids, [g.advance + size * 0.3, pitch], [g.advance / 2.0, -size * 0.36], 0.0);
            let fill = build::shape_fill(&mut ids, [1.0, 1.0, 1.0, 1.0]);
            let grp = build::shape_group(&mut ids, "Window", vec![rect, fill]);
            if let Some(c) = matte.props.sub_mut("contents") {
                c.children.push(grp.into());
            }
            // The strip: 0-9 rows that roll up through the window (layer motion, so it blurs).
            let mut l = build::layer(proj, &comp, names.get(n).map(String::as_str).unwrap_or("Digit"), LayerSource::Text, (comp.width, comp.height), None);
            timing(&mut l);
            l.track_matte = Some(TrackMatte { layer: matte.id, kind: MatteKind::Alpha });
            if let Some(pr) = l.props.prop_mut("text/sourceText") {
                pr.value = KV::Text(Box::new(strip.clone()));
            }
            if let Some(pr) = l.props.prop_mut("transform/opacity") {
                pr.expr = Some(Expression { text: "hasParent ? parent.transform.opacity : value".into(), enabled: true });
            }
            let base = [g.origin.x - so[0], g.origin.y - so[1]];
            let to = -pitch * (10.0 * cycles as f64 + d);
            if let Some(pr) = l.props.prop_mut("transform/position") {
                let ks = [(*ka, KV::Vec3([base[0], base[1], 0.0])), (*kb, KV::Vec3([base[0], base[1] + to, 0.0]))];
                pe::set_keys(pr, &ks, &[curve]);
            }
            proj.next_id = next;
            let (sid, mid) = (l.id, matte.id);
            let c = proj.comp_mut(cid).ok_or(EngineError::NoComp)?;
            let at = at.min(c.layers.len());
            c.layers.insert(at, l);
            c.layers.insert(at, matte);
            made.push(json!({"strip": sid.0, "matte": mid.0}));
            strips.push(sid);
        }
        // The original keeps its other characters; its digits are hidden under the strips.
        let mut next = proj.next_id;
        let mut ids = Ids(&mut next);
        let l = layer_mut(proj, cid, lid)?;
        if let Some(pr) = l.props.prop_mut("text/sourceText")
            && let KV::Text(d) = &mut pr.value
        {
            d.set_attr("figureWidth", &json!("tabular"), None).map_err(|e| bad(C, e))?;
        }
        let sel = expression_selector(&mut ids, 0, hide_expr.clone());
        let g = animator(&mut ids, "Digit Roll Hide", per_char_3d(l), &[("opacity", &[0.0])], Some(sel));
        push_animator(l, g)?;
        proj.next_id = next;
        st.selected_layers = vec![lid];
        if mb {
            pe::motion_blur(proj.comp_mut(cid).ok_or(EngineError::NoComp)?, &strips, pe::SHUTTER);
        }
        Ok(made)
    })?;
    let last = digits.len().saturating_sub(1) as f64;
    Ok(json!({"preset": "digitRoll", "layers": made, "end": x.start.seconds() + x.secs(last * stagger + frames)}))
}
