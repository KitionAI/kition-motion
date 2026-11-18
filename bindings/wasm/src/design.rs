//! Design advice for an agent: what a motion designer would point out in a composition, each with
//! the commands that fix it. khp-bench's generation runs found agents leave a kit's output at its
//! defaults (small, thin, flat type on a plain background) where the same agents writing code add
//! hierarchy, glow and gradients; and that they act on advice given at the step that needs it,
//! not on what a search could find. `_inspect` lists the advice for the active composition, and
//! the text commands answer it for the text they made or changed.
//!
//! Advice is not a diagnostic: nothing here is wrong, only plainer than a designer would leave it.

use kition_kit::diagnostics::{Diagnostic, Severity, Target};
use kition_motion_project::keyframe::TextDoc;
use kition_motion_project::{Comp, ItemId, ItemKind, Layer, LayerId, LayerSource, Project};
use kition_motion_time::Tick;
use serde_json::{Value, json};

/// The largest text under this share of the frame height is a small title (8–12% is usual).
const TITLE_SHARE: f64 = 0.07;
/// Text under this share of the frame height is hard to read in a video.
const MIN_SHARE: f64 = 0.03;
/// The largest text should be at least this many times the next size.
const HIERARCHY: f64 = 1.6;
/// WCAG's contrast ratio for normal text.
const CONTRAST: f64 = 4.5;
/// WCAG's contrast ratio for large text: 24 px, or 18.66 px bold (18 pt, 14 pt bold).
const LARGE_CONTRAST: f64 = 3.0;

/// The contrast `doc` needs: WCAG's for large text when it is large, else for normal text.
fn needed_contrast(doc: &TextDoc) -> f64 {
    let large = doc.size >= 24.0 || (doc.size >= 18.66 && heavy(doc, 700));
    if large { LARGE_CONTRAST } else { CONTRAST }
}

/// One piece of advice: a stable code, what a designer would say, and commands to try. A command
/// written `id {json}` runs as it stands; one written with bare names shows the shape to fill in.
fn advice<S: AsRef<str>>(code: &str, message: String, try_: &[S]) -> Value {
    json!({ "code": code, "message": message, "try": try_.iter().map(AsRef::as_ref).collect::<Vec<_>>() })
}

/// A command to try, ready to run: `id {params}`.
fn ready((id, params): (&str, Value)) -> String {
    format!("{id} {params}")
}

fn hex(c: [f32; 3]) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

/// `c` moved toward `to` by `t`.
fn mix(c: [f32; 3], to: [f32; 3], t: f32) -> [f32; 3] {
    [c[0] + (to[0] - c[0]) * t, c[1] + (to[1] - c[1]) * t, c[2] + (to[2] - c[2]) * t]
}

/// Depth for a flat background `back` (a full-frame layer): a radial ramp of its own colour, lit
/// above the centre and falling off to the edges, so the frame keeps its colour; a vignette.
/// Following the advice as it was (a Gradient Ramp made radial) drew a black-to-white ramp from
/// the top edge over the background (video plan P0).
fn background_depth(comp: &Comp, back: LayerId, colour: [f32; 3]) -> Vec<(&'static str, Value)> {
    let (w, h) = (f64::from(comp.width), f64::from(comp.height));
    let light = mix(colour, [1.0, 1.0, 1.0], 0.18);
    let dark = mix(colour, [0.0, 0.0, 0.0], 0.55);
    vec![
        (
            "effect.apply",
            json!({ "effect": "Gradient Ramp", "layers": [back.0], "params": {
                "shape": 1, "start": [w / 2.0, h * 0.4], "startColor": hex(light), "end": [w / 2.0 + w * 0.65, h * 0.4], "endColor": hex(dark)
            } }),
        ),
        ("effect.apply", json!({ "effect": "CC Vignette", "layers": [back.0], "params": { "amount": 60 } })),
    ]
}

/// Depth for flat elements: a soft glow in `accent` and a shadow, sized for the frame height.
fn element_depth(comp: &Comp, layers: &[LayerId], accent: [f32; 3]) -> Vec<(&'static str, Value)> {
    let h = f64::from(comp.height);
    let ids: Vec<u64> = layers.iter().map(|l| l.0).collect();
    vec![
        ("layer.style.add", json!({ "style": "outerGlow", "layers": ids, "params": { "size": (h * 0.035).round(), "opacity": 60, "color": hex(accent) } })),
        (
            "layer.style.add",
            json!({ "style": "dropShadow", "layers": ids, "params": { "distance": (h * 0.012).round(), "size": (h * 0.03).round(), "opacity": 45 } }),
        ),
    ]
}

/// The comp's background layer, when it is one a ramp can go on: the backmost solid or shape.
fn background(comp: &Comp) -> Option<&Layer> {
    comp.layers.last().filter(|l| matches!(l.source, LayerSource::Solid { .. } | LayerSource::Shape))
}

/// Text and shape elements in front of the background with no shadow or glow, when that is all of
/// them (one element with depth is enough to lift the frame).
fn flat_elements(comp: &Comp) -> Vec<&Layer> {
    let backmost = comp.layers.last().map(|l| l.id);
    let elements: Vec<&Layer> = comp.layers.iter().filter(|l| Some(l.id) != backmost && matches!(l.source, LayerSource::Text | LayerSource::Shape)).collect();
    if elements.iter().any(|l| deep(l)) { vec![] } else { elements }
}

/// The text a layer shows (its Source Text), when it is a text layer.
fn text(layer: &Layer) -> Option<&TextDoc> {
    if !matches!(layer.source, LayerSource::Text) {
        return None;
    }
    layer.props.prop("text/sourceText").and_then(|p| p.value.as_text())
}

/// The words a style or an effect is known by, lower case, for spotting depth and gradients.
fn looks(layer: &Layer) -> String {
    let mut out = String::new();
    for group in [layer.effects(), layer.layer_styles().filter(|g| g.enabled)].into_iter().flatten() {
        for child in &group.children {
            out.push_str(&child.name().to_ascii_lowercase());
            out.push(' ');
            out.push_str(&child.match_id().to_ascii_lowercase());
            out.push(' ');
        }
    }
    out
}

/// Whether anything on the layer moves: keyframes anywhere, or text animators.
/// Whether the layer moves: keyframes, an enabled expression (a count-up, a typewriter) or a
/// text animator.
fn animated(layer: &Layer) -> bool {
    let mut moves = false;
    layer.props.walk("", &mut |_, p| moves |= !p.keys.is_empty() || p.has_expression());
    moves || layer.props.group("text/animators").is_some_and(|g| !g.children.is_empty())
}

/// Whether two layers are on screen together at some time.
fn together(a: &Layer, b: &Layer) -> bool {
    a.in_point < b.out_point && b.in_point < a.out_point
}

/// The next largest text on screen together with `title`.
fn rival<'a>(texts: &[&'a (&'a Layer, &'a TextDoc)], title: &Layer) -> Option<&'a (&'a Layer, &'a TextDoc)> {
    texts.iter().copied().filter(|(l, _)| l.id != title.id && together(l, title)).max_by(|a, b| a.1.size.total_cmp(&b.1.size))
}

/// The first solid fill colour of a shape layer's contents.
fn shape_fill(g: &kition_motion_project::PropGroup) -> Option<[f32; 3]> {
    for c in &g.children {
        let Some(sub) = c.as_group() else { continue };
        if sub.match_id == "fill"
            && let Some(kition_motion_project::keyframe::Value::Color(c)) = sub.prop("color").map(|p| &p.value)
        {
            return Some([c[0] as f32, c[1] as f32, c[2] as f32]);
        }
        if let Some(found) = shape_fill(sub) {
            return Some(found);
        }
    }
    None
}

/// The colour right behind the middle of `text` while it is on screen (halfway through its
/// time, when a plate that slides in under it has arrived): the solid and filled shape layers
/// below it that cover that point, composited with their opacity over the backdrop.
fn behind(project: &Project, comp: &Comp, text: &Layer) -> [f32; 3] {
    let Some(cid) = project.items.iter().find(|(_, item)| matches!(&item.kind, ItemKind::Comp(c) if std::ptr::eq(c.as_ref(), comp))).map(|(id, _)| *id) else {
        return backdrop(project, comp);
    };
    let end = text.out_point.min(comp.duration).max(text.in_point);
    let at = Tick(text.in_point.0 + (end.0 - text.in_point.0) / 2);
    let ctx = kition_motion_engine::render::EvalCtx::new(project, cid, comp, at);
    let Some([x0, y0, x1, y1]) = kition_motion_engine::render::content_bounds(&ctx, text) else { return backdrop(project, comp) };
    let centre = ctx.layer_to_comp(text).0.apply(kition_motion_engine::geom::vec2((x0 + x1) / 2.0, (y0 + y1) / 2.0));
    // The covering layers from the top down to the first opaque one.
    let mut stack = vec![];
    for layer in comp.layers.iter().skip_while(|l| l.id != text.id).skip(1).filter(|l| l.is_active_at(at)) {
        let colour = match &layer.source {
            LayerSource::Solid { item } => match project.items.get(item).map(|i| &i.kind) {
                Some(ItemKind::Solid(solid)) => Some(solid.color),
                _ => None,
            },
            LayerSource::Shape => layer.props.group("contents").and_then(shape_fill),
            _ => None,
        };
        let Some(colour) = colour else { continue };
        let Some([x0, y0, x1, y1]) = kition_motion_engine::render::content_bounds(&ctx, layer) else { continue };
        let Some(inverse) = ctx.layer_to_comp(layer).0.inverse() else { continue };
        let p = inverse.apply(centre);
        if (x0..=x1).contains(&p.x) && (y0..=y1).contains(&p.y) {
            let opacity = ctx.opacity(layer).clamp(0.0, 1.0) as f32;
            stack.push((colour, opacity));
            if opacity >= 0.999 {
                break;
            }
        }
    }
    let under = if stack.last().is_some_and(|(_, o)| *o >= 0.999) { [0.0; 3] } else { backdrop(project, comp) };
    stack.iter().rev().fold(under, |below, (colour, opacity)| std::array::from_fn(|i| below[i] + (colour[i] - below[i]) * opacity))
}

fn luminance(c: [f32; 3]) -> f64 {
    let ch = |v: f32| {
        let v = f64::from(v.clamp(0.0, 1.0));
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

fn contrast(a: [f32; 3], b: [f32; 3]) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The colour behind the text: the bottom solid layer when there is one, else the background.
fn backdrop(project: &Project, comp: &Comp) -> [f32; 3] {
    comp.layers
        .iter()
        .rev()
        .find_map(|layer| match &layer.source {
            LayerSource::Solid { item } => match project.items.get(item).map(|i| &i.kind) {
                Some(ItemKind::Solid(solid)) => Some(solid.color),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or(comp.background)
}

/// Advice on one text layer: its size against the frame and its contrast with the backdrop.
fn on_text(project: &Project, comp: &Comp, layer: &Layer, doc: &TextDoc) -> Vec<Value> {
    let mut out = vec![];
    let height = f64::from(comp.height.max(1));
    let name = &layer.name;
    if doc.size < height * MIN_SHARE {
        out.push(advice(
            "design.small-text",
            format!(
                "\"{name}\" is {:.0} px, {:.1}% of the frame height: hard to read in a video. Keep text at least {:.0} px ({:.0}% of {height:.0}).",
                doc.size,
                doc.size / height * 100.0,
                (height * MIN_SHARE).ceil(),
                MIN_SHARE * 100.0
            ),
            &["layer.setText {layer, size}"],
        ));
    }
    let font = kition_motion_text::fonts::resolve(&doc.font, &doc.style);
    if font.missing {
        out.push(advice(
            "design.missing-font",
            format!(
                "\"{name}\" asks for the font \"{}\", which is not installed here: it is drawn with a fallback, so it will not look as chosen. Pick an installed family.",
                doc.font
            ),
            &["text.fonts {query}", "layer.setText {layer, font, style}"],
        ));
    }
    let back = behind(project, comp, layer);
    let fill = [doc.fill[0], doc.fill[1], doc.fill[2]];
    let ratio = contrast(fill, back);
    let needed = needed_contrast(doc);
    if doc.apply_fill && ratio < needed {
        out.push(advice(
            "design.low-contrast",
            format!("\"{name}\" has a contrast of {ratio:.1}:1 with what is behind it; {needed}:1 or more reads clearly at its size. Lighten or darken the text or the background."),
            &["layer.setText {layer, fill}", "layer.style.add {style: dropShadow, layers}"],
        ));
    }
    out
}

/// Whether a text style is at least `weight` (500 medium, 600 semibold, 700 bold), by its name.
fn heavy(doc: &TextDoc, weight: u16) -> bool {
    let style = doc.style.to_ascii_lowercase().replace([' ', '-'], "");
    let named = if ["black", "heavy", "extrabold", "ultrabold", "w8", "w9"].iter().any(|w| style.contains(w)) {
        900
    } else if style.contains("semibold") || style.contains("demibold") || style.contains("w6") {
        600
    } else if style.contains("bold") || style.contains("w7") {
        700
    } else if style.contains("medium") || style.contains("w5") {
        500
    } else {
        400
    };
    doc.faux_bold || named >= weight
}

/// Whether a layer's effects give the frame depth beyond a flat fill: a vignette, light, glow,
/// grain or noise, or a radial Gradient Ramp. A linear ramp alone is still flat: judges called
/// it "flat linear background" (khp-bench).
fn textured(layer: &Layer) -> bool {
    if ["vignette", "glow", "sheen", "light", "noise", "grain", "lens", "fractal"].iter().any(|w| looks(layer).contains(w)) {
        return true;
    }
    layer.effects().is_some_and(|g| {
        g.children
            .iter()
            .filter_map(|c| c.as_group())
            .any(|e| e.match_id.contains("gradientramp") && e.prop("shape").is_some_and(|p| p.value == kition_motion_project::keyframe::Value::Enum(1)))
    })
}

/// The glow colour: the largest text's fill, or a light tint of the background.
fn accent(texts: &[(&Layer, &TextDoc)], back: [f32; 3]) -> [f32; 3] {
    texts
        .iter()
        .max_by(|a, b| a.1.size.total_cmp(&b.1.size))
        .map(|(_, t)| [t.fill[0], t.fill[1], t.fill[2]])
        .filter(|c| contrast(*c, back) >= 2.0)
        .unwrap_or_else(|| mix(back, [1.0, 1.0, 1.0], 0.6))
}

/// Whether a layer has a shadow, a glow or a bevel.
fn deep(layer: &Layer) -> bool {
    let l = looks(layer);
    ["shadow", "glow", "bevel", "lift"].iter().any(|w| l.contains(w))
}

/// What a designer would point out in composition `comp`, most telling first. The order follows
/// what separated KHP videos from the code-made ones in khp-bench's judgements (criticisms on one
/// side only): a flat frame (53 against 1), no depth, thin type, low contrast; "small text" was
/// said of both sides alike and comes last.
pub fn review(project: &Project, comp: &Comp) -> Vec<Value> {
    let texts: Vec<(&Layer, &TextDoc)> = comp.layers.iter().filter_map(|l| text(l).map(|t| (l, t))).collect();
    if texts.is_empty() {
        return vec![];
    }
    let height = f64::from(comp.height.max(1));
    let mut out = vec![];
    if !comp.layers.iter().any(textured) {
        let try_ = match background(comp) {
            Some(back) => background_depth(comp, back.id, backdrop(project, comp)).into_iter().map(ready).collect(),
            None => vec![format!(
                "layer.newSolid {{\"name\": \"Background\", \"color\": \"{}\"}}, moved to the bottom, then a radial Gradient Ramp on it",
                hex(comp.background)
            )],
        };
        out.push(advice(
            "design.flat-background",
            "The frame is flat: a plain colour or a plain linear gradient, with no vignette, radial light or grain. Judges call such frames flat and template-like. \
             Give the background depth: a radial ramp of its own colour, lighter behind the subject and darker at the edges, or a vignette. The commands below run as written."
                .into(),
            &try_,
        ));
    }
    // Text and shape elements (not the backmost layer, the background) without shadow or glow.
    let flat_layers = flat_elements(comp);
    let flat: Vec<&str> = flat_layers.iter().map(|l| l.name.as_str()).collect();
    if !flat.is_empty() {
        out.push(advice(
            "design.no-depth",
            format!(
                "No element has a glow or a shadow ({}), so everything sits flat on the background. A soft outer glow on the key elements (numbers, rings, icons, the title) and a shadow under cards give depth.",
                flat.iter().take(6).map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", ")
            ),
            &element_depth(comp, &flat_layers.iter().map(|l| l.id).collect::<Vec<_>>(), accent(&texts, backdrop(project, comp)))
                .into_iter()
                .map(ready)
                .collect::<Vec<_>>(),
        ));
    }
    let largest = texts.iter().map(|(_, t)| t.size).fold(0.0, f64::max);
    let thin: Vec<&str> = texts.iter().filter(|(_, t)| if t.size >= largest { !heavy(t, 600) } else { !heavy(t, 500) }).map(|(l, _)| l.name.as_str()).collect();
    if !thin.is_empty() {
        out.push(advice(
            "design.thin-type",
            format!(
                "{} {} set in a light or regular weight; on video, thin type reads faint. Titles look finished in SemiBold or Bold, secondary text in Medium or heavier.",
                thin.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", "),
                if thin.len() == 1 { "is" } else { "are" }
            ),
            &["layer.setText {layer, style: SemiBold}", "text.fonts {query} (the styles a family has)"],
        ));
    }
    for (layer, doc) in &texts {
        out.extend(on_text(project, comp, layer, doc).into_iter().filter(|a| a["code"] != "design.small-text"));
    }
    let mut sizes: Vec<f64> = texts.iter().map(|(_, t)| t.size).collect();
    sizes.sort_by(|a, b| b.total_cmp(a));
    // Title and subtitle compete only when they are on screen together.
    let ranked: Vec<&(&Layer, &TextDoc)> = texts.iter().collect();
    let title = ranked.iter().copied().max_by(|a, b| a.1.size.total_cmp(&b.1.size));
    if let Some((title, title_doc)) = title
        && let Some((_, next)) = rival(&ranked, title)
        && title_doc.size < next.size * HIERARCHY
    {
        out.push(advice(
            "design.weak-hierarchy",
            format!(
                "The two largest texts on screen together are {:.0} and {:.0} px: too close to read as title and subtitle. Make the title at least {HIERARCHY}× the next size, or bolder.",
                title_doc.size, next.size
            ),
            &["layer.setText {layer, size, style}"],
        ));
    }
    if sizes[0] < height * TITLE_SHARE {
        out.push(advice(
            "design.small-title",
            format!(
                "The largest text is {:.0} px, {:.1}% of the frame height. Titles in motion graphics are usually 8–12% ({:.0}–{:.0} px here), in a bold weight.",
                sizes[0],
                sizes[0] / height * 100.0,
                height * 0.08,
                height * 0.12
            ),
            &["layer.setText {layer, size, style: Bold}"],
        ));
    }
    let still: Vec<&str> = texts.iter().filter(|(l, _)| !animated(l)).map(|(l, _)| l.name.as_str()).collect();
    if !still.is_empty() {
        out.push(advice(
            "design.static-text",
            format!(
                "{} never move{}. Text presets give per-word or per-character entrances in one step (text.presets lists them: blurSlide, letterpress, typewriter, …).",
                still.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", "),
                if still.len() == 1 { "s" } else { "" }
            ),
            &["layer.applyTextPreset {layer, preset}", "text.presets {}"],
        ));
    }
    for (layer, doc) in &texts {
        out.extend(on_text(project, comp, layer, doc).into_iter().filter(|a| a["code"] == "design.small-text"));
    }
    out
}

/// The advice as `motion_check` reports it, for the active composition `comp`: Info (nothing is
/// wrong, only plainer than a designer would leave it), one diagnostic per layer where the advice
/// is about one, each fix a command that runs as written and clears its code. In khp-bench's video
/// runs every agent called `motion_check` before exporting; none read `_inspect`'s advice or the
/// guide, so advice given only there never reached them.
pub fn diagnostics(project: &Project, comp_id: ItemId, comp: &Comp) -> Vec<Diagnostic> {
    let texts: Vec<(&Layer, &TextDoc)> = comp.layers.iter().filter_map(|l| text(l).map(|t| (l, t))).collect();
    if texts.is_empty() {
        return vec![];
    }
    let height = f64::from(comp.height.max(1));
    let back = backdrop(project, comp);
    let whole = || Target { path: Some(format!("item:{}", comp_id.0)), ..Target::default() };
    let on = |l: &Layer| Target { path: Some(format!("comp:{}/layer:{}", comp_id.0, l.id.0)), ..Target::default() };
    let info = |code: &str, message: String| Diagnostic::new(code, Severity::Info, message);
    let mut out = vec![];
    if !comp.layers.iter().any(textured) {
        let found = info(
            "motion.flat-background",
            "The frame is flat (one colour or a linear gradient, no radial light or vignette), which reads as plain; a radial ramp of the background's own colour, lighter behind the subject, gives it depth.".into(),
        )
        .on(whole());
        out.push(match background(comp).and_then(|b| background_depth(comp, b.id, back).into_iter().next()) {
            Some((command, params)) => found.fix(command, Some(params)),
            None => found,
        });
    }
    let flat = flat_elements(comp);
    if let Some((command, params)) = element_depth(comp, &flat.iter().map(|l| l.id).collect::<Vec<_>>(), accent(&texts, back)).into_iter().next()
        && !flat.is_empty()
    {
        let names = flat.iter().take(6).map(|l| format!("\"{}\"", l.name)).collect::<Vec<_>>().join(", ");
        out.push(
            info(
                "motion.no-depth",
                format!(
                    "No element has a glow or a shadow ({names}), so everything sits flat on the background; a soft outer glow sized for the frame lifts them."
                ),
            )
            .on(whole())
            .fix(command, Some(params)),
        );
    }
    let mut by_size: Vec<&(&Layer, &TextDoc)> = texts.iter().collect();
    by_size.sort_by(|a, b| b.1.size.total_cmp(&a.1.size));
    let (title, title_doc) = *by_size[0];
    if title_doc.size < height * TITLE_SHARE {
        let size = (height * 0.1).round();
        out.push(
            info(
                "motion.small-title",
                format!("The largest text, \"{}\", is {:.0} px ({:.1}% of the frame height); titles in motion graphics are 8–12% ({:.0}–{:.0} px here), in a bold weight.", title.name, title_doc.size, title_doc.size / height * 100.0, height * 0.08, height * 0.12),
            )
            .on(on(title))
            .fix("layer.setText", Some(json!({ "layer": title.id.0, "size": size, "style": "Bold" }))),
        );
    } else if let Some((_, next)) = rival(&by_size, title)
        && title_doc.size < next.size * HIERARCHY
    {
        let size = (next.size * (HIERARCHY + 0.1)).ceil();
        out.push(
            info(
                "motion.weak-hierarchy",
                format!(
                    "The two largest texts are {:.0} and {:.0} px, too close to read as title and subtitle; make \"{}\" at least {HIERARCHY}× the next size.",
                    title_doc.size, next.size, title.name
                ),
            )
            .on(on(title))
            .fix("layer.setText", Some(json!({ "layer": title.id.0, "size": size }))),
        );
    }
    for (layer, doc) in &texts {
        let is_title = layer.id == title.id;
        if !heavy(doc, if is_title { 600 } else { 500 }) {
            let style = if is_title { "Bold" } else { "Medium" };
            out.push(
                info(
                    "motion.thin-type",
                    format!("\"{}\" is set in a light or regular weight, which reads faint on video; use {style} or heavier.", layer.name),
                )
                .on(on(layer))
                .fix("layer.setText", Some(json!({ "layer": layer.id.0, "style": style }))),
            );
        }
        let fill = [doc.fill[0], doc.fill[1], doc.fill[2]];
        let under = behind(project, comp, layer);
        let ratio = contrast(fill, under);
        let needed = needed_contrast(doc);
        if doc.apply_fill && ratio < needed {
            let better = if contrast([1.0; 3], under) >= contrast([0.0; 3], under) { "#ffffff" } else { "#000000" };
            out.push(
                info(
                    "motion.low-contrast",
                    format!("\"{}\" has a contrast of {ratio:.1}:1 with what is behind it; {needed}:1 or more reads clearly at its size.", layer.name),
                )
                .on(on(layer))
                .fix("layer.setText", Some(json!({ "layer": layer.id.0, "fill": better }))),
            );
        }
        if doc.size < height * MIN_SHARE {
            let size = (height * MIN_SHARE).ceil();
            out.push(
                info("motion.small-text", format!("\"{}\" is {:.0} px, hard to read in a video; keep text at least {size:.0} px here.", layer.name, doc.size))
                    .on(on(layer))
                    .fix("layer.setText", Some(json!({ "layer": layer.id.0, "size": size }))),
            );
        }
        if kition_motion_text::fonts::resolve(&doc.font, &doc.style).missing {
            out.push(
                info(
                    "motion.missing-font",
                    format!(
                        "\"{}\" asks for the font \"{}\", which is not installed, so it is drawn with a fallback; text.fonts lists the installed families.",
                        layer.name, doc.font
                    ),
                )
                .on(on(layer)),
            );
        }
        if !animated(layer) {
            let preset = if is_title {
                json!({ "layer": layer.id.0, "preset": "blurSlide" })
            } else {
                json!({ "layer": layer.id.0, "preset": "blurSlide", "variant": "subtitle" })
            };
            out.push(
                info("motion.static-text", format!("\"{}\" never moves; a text preset gives it a per-word entrance in one step.", layer.name))
                    .on(on(layer))
                    .fix("layer.applyTextPreset", Some(preset)),
            );
        }
    }
    out
}

/// Advice on the text layer a command made or changed, for that command's answer.
pub fn on_layer(project: &Project, comp: &Comp, layer: LayerId) -> Vec<Value> {
    let Some(l) = comp.layer(layer) else { return vec![] };
    let Some(doc) = text(l) else { return vec![] };
    let mut out = on_text(project, comp, l, doc);
    let height = f64::from(comp.height.max(1));
    let largest = comp.layers.iter().filter_map(text).map(|t| t.size).fold(0.0, f64::max);
    if doc.size >= largest && doc.size < height * TITLE_SHARE {
        out.push(advice(
            "design.small-title",
            format!(
                "This is the largest text so far, at {:.0} px; if it is the title, titles in motion graphics are usually {:.0}–{:.0} px at this frame height, in a bold weight.",
                doc.size,
                height * 0.08,
                height * 0.12
            ),
            &["layer.setText {layer, size, style: Bold}"],
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::MotionDocument;
    use kition_kit::KitDocument;
    use serde_json::json;

    fn codes(advice: &serde_json::Value) -> Vec<String> {
        advice.as_array().into_iter().flatten().filter_map(|a| a["code"].as_str().map(str::to_owned)).collect()
    }

    /// Every command the advice suggests exists and runs as written (khp-bench: a suggestion that
    /// fails costs the agent a round trip and its trust in the advice).
    #[test]
    fn the_suggested_commands_run() {
        let mut doc = MotionDocument::create("check.kmotion", 1920, 1080).unwrap();
        let back = doc.execute("layer.newSolid", json!({ "name": "Back", "color": "#101828" })).unwrap()["layer"].clone();
        let title = doc.execute("layer.newText", json!({ "text": "Launch", "size": 120 })).unwrap()["layer"].clone();
        for (command, params) in [
            ("effect.apply", json!({ "effect": "Gradient Ramp", "layers": [back] })),
            ("effect.apply", json!({ "effect": "CC Vignette", "layers": [back] })),
            ("effect.apply", json!({ "effect": "Add Grain", "layers": [back] })),
            ("prop.set", json!({ "layer": back, "path": "effects/#1/shape", "value": 1 })),
            ("light.sheen", json!({ "layer": title })),
            ("layer.liftShadow", json!({ "layer": title })),
            ("layer.style.add", json!({ "style": "outerGlow", "layers": [title] })),
            ("layer.style.add", json!({ "style": "dropShadow", "layers": [title] })),
            ("layer.applyTextPreset", json!({ "layer": title, "preset": "blurSlide" })),
            ("text.presets", json!({})),
            ("text.fonts", json!({ "query": "Inter" })),
            ("layer.setText", json!({ "layer": title, "size": 110, "style": "Bold", "fill": "#ffffff" })),
        ] {
            doc.execute(command, params.clone()).unwrap_or_else(|e| panic!("{command} {params}: {e}"));
        }
    }

    /// The advice's ready commands (`id {json}`) for `code`, as `(id, params)`.
    fn ready(advice: &serde_json::Value, code: &str) -> Vec<(String, serde_json::Value)> {
        let found = advice.as_array().into_iter().flatten().find(|a| a["code"] == code).unwrap_or_else(|| panic!("no {code}: {advice}"));
        found["try"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t.as_str()?.split_once(' '))
            .filter_map(|(id, params)| Some((id.to_owned(), serde_json::from_str(params).ok()?)))
            .collect()
    }

    /// Mean of `f` over the RGBA8 pixels of `frame` (`width` wide) in the box `[x0, y0, x1, y1)`.
    fn mean(frame: &[u8], width: usize, [x0, y0, x1, y1]: [usize; 4], f: impl Fn(&[u8]) -> f64) -> f64 {
        let mut sum = 0.0;
        for y in y0..y1 {
            for x in x0..x1 {
                let i = (y * width + x) * 4;
                sum += f(&frame[i..i + 4]);
            }
        }
        sum / ((x1 - x0) * (y1 - y0)) as f64
    }

    fn luma(px: &[u8]) -> f64 {
        0.2126 * f64::from(px[0]) + 0.7152 * f64::from(px[1]) + 0.0722 * f64::from(px[2])
    }

    /// Video plan P0: following the depth advice exactly as written gives the frame depth and keeps
    /// its colour (it drew a black-to-white ramp over the background), and the suggested glow is
    /// big enough to see at 1080p (at the style's default size it barely changes the frame).
    #[test]
    fn following_the_depth_advice_as_written_gives_depth_and_keeps_the_colour() {
        let (w, h) = (480usize, 270usize);
        let make = || {
            let mut doc = MotionDocument::create("depth.kmotion", 1920, 1080).unwrap();
            doc.execute("layer.newSolid", json!({ "name": "Back", "color": "#1e2a4a" })).unwrap();
            let title = doc.execute("layer.newText", json!({ "text": "Launch", "size": 140, "fill": "#ffffff", "style": "Bold" })).unwrap()["layer"].clone();
            (doc, title)
        };
        let (mut doc, title) = make();
        let advice = doc.inspect().unwrap()["design"].clone();

        let background = ready(&advice, "design.flat-background");
        assert!(!background.is_empty(), "{advice}");
        for (id, params) in &background {
            doc.execute(id, params.clone()).unwrap_or_else(|e| panic!("{id} {params}: {e}"));
        }
        assert!(!codes(&doc.inspect().unwrap()["design"]).contains(&"design.flat-background".to_string()));
        let frame = doc.render(0.0, 0.0, 0.25, w as u32, h as u32).unwrap();
        let (centre, corner) = (mean(&frame, w, [200, 40, 280, 70], luma), mean(&frame, w, [0, 0, 30, 30], luma));
        assert!(centre > corner + 12.0, "lit behind the subject, darker at the edges: {centre:.0} vs {corner:.0}");
        let (red, blue) = (mean(&frame, w, [0, 0, w, h], |p| f64::from(p[0])), mean(&frame, w, [0, 0, w, h], |p| f64::from(p[2])));
        assert!(blue > red + 15.0 && centre < 140.0, "still the navy background, not a grey ramp: red {red:.0}, blue {blue:.0}, centre {centre:.0}");

        // The glow, as suggested and at the style's default size: how much each changes around the title.
        let around = [120, 95, 360, 175];
        let change = |params: &serde_json::Value| {
            let (mut doc, _) = make();
            let before = doc.render(0.0, 0.0, 0.25, w as u32, h as u32).unwrap();
            doc.execute("layer.style.add", params.clone()).unwrap();
            let after = doc.render(0.0, 0.0, 0.25, w as u32, h as u32).unwrap();
            let diff: Vec<u8> = before.iter().zip(&after).map(|(a, b)| a.abs_diff(*b)).collect();
            mean(&diff, w, around, |p| f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2]))
        };
        let depth = ready(&advice, "design.no-depth");
        let glow = depth.iter().find(|(id, p)| id == "layer.style.add" && p["style"] == "outerGlow").unwrap_or_else(|| panic!("{advice}"));
        assert_eq!(glow.1["layers"], json!([title]), "{advice}");
        let (suggested, default) = (change(&glow.1), change(&json!({ "style": "outerGlow", "layers": [title] })));
        assert!(suggested > default * 3.0 && suggested > 4.0, "suggested glow {suggested:.1}, default {default:.1}");
        for (id, params) in &depth {
            doc.execute(id, params.clone()).unwrap_or_else(|e| panic!("{id} {params}: {e}"));
        }
        assert!(!codes(&doc.inspect().unwrap()["design"]).contains(&"design.no-depth".to_string()));
    }

    /// A linear ramp alone is still a flat frame (judges: "flat linear background"); a radial one,
    /// or a vignette, is not.
    #[test]
    fn a_linear_ramp_is_flat_and_a_radial_one_is_not() {
        let mut doc = MotionDocument::create("ramp.kmotion", 1920, 1080).unwrap();
        let back = doc.execute("layer.newSolid", json!({ "name": "Back", "color": "#101828" })).unwrap()["layer"].clone();
        doc.execute("layer.newText", json!({ "text": "Launch", "size": 120 })).unwrap();
        doc.execute("effect.apply", json!({ "effect": "Gradient Ramp", "layers": [back] })).unwrap();
        assert!(codes(&doc.inspect().unwrap()["design"]).contains(&"design.flat-background".to_string()));
        doc.execute("prop.set", json!({ "layer": back, "path": "effects/#1/shape", "value": 1 })).unwrap();
        assert!(!codes(&doc.inspect().unwrap()["design"]).contains(&"design.flat-background".to_string()));
    }

    /// khp-bench: a plain composition (small, close sizes, low contrast, no depth, no motion, flat
    /// background) gets every piece of advice; a designed one gets none.
    #[test]
    fn a_plain_composition_gets_advice_and_a_designed_one_none() {
        let mut doc = MotionDocument::create("plain.kmotion", 1920, 1080).unwrap();
        doc.execute("layer.newSolid", json!({ "name": "Back", "color": "#202020" })).unwrap();
        let made = doc.execute("layer.newText", json!({ "text": "Launch", "size": 44, "fill": "#333333" })).unwrap();
        assert!(codes(&made["design"]).contains(&"design.small-title".to_string()), "{made}");
        assert!(codes(&made["design"]).contains(&"design.low-contrast".to_string()), "{made}");
        doc.execute("layer.newText", json!({ "text": "Tagline", "size": 30, "fill": "#ffffff" })).unwrap();
        let advice = &doc.inspect().unwrap()["design"];
        for code in [
            "design.small-title",
            "design.weak-hierarchy",
            "design.low-contrast",
            "design.flat-background",
            "design.no-depth",
            "design.static-text",
            "design.thin-type",
        ] {
            assert!(codes(advice).contains(&code.to_string()), "{code}: {advice}");
        }

        let mut doc = MotionDocument::create("designed.kmotion", 1920, 1080).unwrap();
        let back = doc.execute("layer.newSolid", json!({ "name": "Back", "color": "#101828" })).unwrap()["layer"].clone();
        doc.execute("effect.apply", json!({ "effect": "Gradient Ramp", "layers": [back] })).unwrap();
        doc.execute("prop.set", json!({ "layer": back, "path": "effects/#1/shape", "value": 1 })).unwrap();
        let title = doc.execute("layer.newText", json!({ "text": "Launch", "size": 120, "fill": "#ffffff", "style": "Bold" })).unwrap();
        assert!(codes(&title["design"]).is_empty(), "{title}");
        let title = title["layer"].clone();
        doc.execute("layer.style.add", json!({ "style": "dropShadow", "layers": [title] })).unwrap();
        doc.execute("layer.applyTextPreset", json!({ "layer": title, "preset": "blurSlide" })).unwrap();
        let sub = doc.execute("layer.newText", json!({ "text": "Tagline", "size": 48, "fill": "#e0e0e0", "style": "Medium" })).unwrap()["layer"].clone();
        doc.execute("layer.applyTextPreset", json!({ "layer": sub, "preset": "blurSlide" })).unwrap();
        let advice = &doc.inspect().unwrap()["design"];
        assert!(codes(advice).is_empty(), "{advice}");
        let missing = doc.execute("layer.setText", json!({ "layer": sub, "font": "No Such Family 123" })).unwrap();
        assert!(codes(&missing["design"]).contains(&"design.missing-font".to_string()), "{missing}");
    }
}
