//! Effect menu and Effect Controls gestures.

use kition_motion_keyframe::Value as KV;
use kition_motion_project::build::Ids;
use kition_motion_project::{GroupKind, LayerId, Node, PropGroup, Uid};
use serde_json::{Value, json};

use super::{CommandSpec, b_p, bad, f_p, has_layers, layer_mut, layers_p, str_p};
use crate::{EngineError, Result, Session, cmd};

/// `effect.plugins.list`: the registered plug-in effects (WebAssembly or Rust).
fn plugins_list(s: &mut Session, _: &Value) -> Result<Value> {
    use kition_motion_effects::plugin;
    let list: Vec<Value> = plugin::plugins()
        .into_iter()
        .map(|spec| {
            let p = plugin::plugin(spec.id);
            let m = p.as_ref().map(|p| p.manifest().clone());
            json!({
                "id": spec.id,
                "name": spec.name,
                "category": spec.category,
                "version": m.as_ref().map(|m| m.version.clone()),
                "author": m.as_ref().map(|m| m.author.clone()),
                "description": m.as_ref().map(|m| m.description.clone()),
                "source": p.map(|p| p.source()),
                "params": spec.params.iter().map(|p| p.id).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({"api": plugin::PLUGIN_API_VERSION, "wasm": s.plugin_loader.is_some(), "plugins": list}))
}

/// `effect.plugins.load`: load WebAssembly effect plug-ins (a file, or every `.wasm` in a
/// folder) through the host's loader ([`Session::plugin_loader`]).
fn plugins_load(s: &mut Session, p: &Value) -> Result<Value> {
    let c = "effect.plugins.load";
    let loader = s.plugin_loader.ok_or_else(|| EngineError::Other("WebAssembly effect plug-ins are not available in this build".into()))?;
    let files: Vec<String> = match (str_p(p, "path"), str_p(p, "folder")) {
        (Some(path), _) => vec![path.to_string()],
        (None, Some(dir)) => {
            s.need_file_system("Loading the plug-ins of a folder")?;
            let mut v: Vec<String> = std::fs::read_dir(dir)
                .map_err(|e| bad(c, format!("cannot list {dir}: {e}")))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wasm")))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            v.sort();
            v
        }
        _ => return Err(bad(c, "give `path` or `folder`")),
    };
    let (mut loaded, mut errors) = (vec![], vec![]);
    for f in &files {
        let r = s.services.read_file(f).map_err(|e| format!("cannot read {f}: {e}")).and_then(|bytes| loader(&bytes, f));
        match r {
            Ok(v) => loaded.push(v),
            Err(e) => errors.push(json!({"path": f, "error": e})),
        }
    }
    if str_p(p, "path").is_some()
        && let Some(e) = errors.first()
    {
        return Err(EngineError::Other(format!("{}: {}", files[0], e["error"].as_str().unwrap_or("failed"))));
    }
    if !loaded.is_empty() {
        s.toast(format!("Loaded {} effect plug-in(s)", loaded.len()));
    }
    Ok(json!({"loaded": loaded, "errors": errors}))
}

/// A parameter of an effect instance by what an agent writes: its match id, its name, or its name
/// without spaces and case (`"blurriness"`, `"Blurriness"`, `"Start Color"`, `"startcolor"`).
pub(crate) fn param_index(g: &PropGroup, key: &str) -> Option<usize> {
    let squash = |x: &str| x.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    let k = squash(key);
    g.children
        .iter()
        .position(|c| c.match_id() == key || c.name().eq_ignore_ascii_case(key))
        .or_else(|| g.children.iter().position(|c| squash(c.match_id()) == k || squash(c.name()) == k))
}

/// The node a parameter key names: a parameter (`amount`), or one in a parameter group by its
/// path as effect.list lists it (`positionsColors/color1`, `transform/scale`).
pub(crate) fn param_node<'g>(g: &'g PropGroup, key: &str) -> Option<&'g Node> {
    let (head, rest) = match key.split_once('/') {
        Some((head, rest)) if param_index(g, key).is_none() => (head, Some(rest)),
        _ => (key, None),
    };
    let node = g.children.get(param_index(g, head)?)?;
    match rest {
        None => Some(node),
        Some(rest) => param_node(node.as_group()?, rest),
    }
}

fn param_node_mut<'g>(g: &'g mut PropGroup, key: &str) -> Option<&'g mut Node> {
    let (head, rest) = match key.split_once('/') {
        Some((head, rest)) if param_index(g, key).is_none() => (head, Some(rest)),
        _ => (key, None),
    };
    let i = param_index(g, head)?;
    let node = g.children.get_mut(i)?;
    match rest {
        None => Some(node),
        Some(rest) => match node {
            Node::Group(sub) => param_node_mut(sub, rest),
            _ => None,
        },
    }
}

/// Sets `params` on a new effect or style instance `g` as prop.set would set them: each value read
/// in its parameter's type, sliders clamped. Done inside the edit that adds the instance, so the
/// instance and its settings are one undo step.
pub(crate) fn set_params(g: &mut PropGroup, params: &serde_json::Map<String, Value>, what: &str) -> std::result::Result<(), String> {
    for (key, v) in params {
        let node = param_node_mut(g, key).ok_or_else(|| format!("{what} has no parameter `{key}`"))?;
        let Node::Prop(pr) = node else {
            return Err(format!("{what} `{key}` is a group: set what is in it by path with prop.set"));
        };
        let mut value = pr.value.coerce_json(v).ok_or_else(|| format!("{what} `{key}`: can't use {v} for a {} property", pr.value.kind_name()))?;
        if let KV::Scalar(x) = value {
            value = KV::Scalar(pr.ui.keep_in_range(x));
        }
        pr.value = value;
    }
    Ok(())
}

/// An effect instance's parameters as the agent can set them: path, name and current value
/// (groups list their children's paths).
pub(crate) fn params_json(g: &PropGroup, prefix: &str) -> Vec<Value> {
    let mut out = vec![];
    for (i, c) in g.children.iter().enumerate() {
        let path = format!("{prefix}/{}", super::query::segment(g, i));
        match c.as_prop() {
            Some(pr) => out.push(json!({"path": path, "name": pr.name, "value": pr.value.to_json()})),
            None => {
                if let Some(sub) = c.as_group() {
                    out.extend(params_json(sub, &path));
                }
            }
        }
    }
    out
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let name = str_p(p, "effect").ok_or_else(|| bad("effect.apply", "missing `effect` (id or name)"))?;
    let spec = kition_motion_effects::lookup(name).ok_or_else(|| bad("effect.apply", format!("unknown effect `{name}`")))?;
    let (cid, ids) = layers_p(s, p)?;
    if ids.is_empty() {
        return Err(bad("effect.apply", "no layer"));
    }
    let sizes: Vec<_> = {
        let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
        ids.iter().filter_map(|id| comp.layer(*id)).map(|l| (l.id, kition_motion_render::effect_bounds(&s.project, comp, l).0)).collect()
    };
    // `params`: the effect's settings in the same step. Every name is checked before the effect
    // is applied, so a wrong one changes nothing and its error lists the right ones.
    let params = match p.get("params") {
        None | Some(Value::Null) => None,
        Some(Value::Object(m)) => Some(m.clone()),
        Some(other) => return Err(bad("effect.apply", format!("`params` is an object of parameter names and values, not {other}"))),
    };
    if let (Some(params), Some((_, size))) = (&params, sizes.first()) {
        let mut scratch = s.project.next_id;
        let mut probe = kition_motion_effects::instantiate(spec, &mut Ids(&mut scratch), spec.name, *size);
        for key in params.keys() {
            if param_node(&probe, key).is_none() {
                let names: Vec<String> = probe.children.iter().map(|c| c.name().to_owned()).take(30).collect();
                return Err(bad("effect.apply", format!("{} has no parameter `{key}`; its parameters are: {}", spec.name, names.join(", "))));
            }
        }
        set_params(&mut probe, params, spec.name).map_err(|e| bad("effect.apply", e))?;
    }
    let uids = s.edit(&format!("Apply {}", spec.name), None, |proj, st| {
        let mut out = vec![];
        for (lid, size) in &sizes {
            let mut next = proj.next_id;
            let l = layer_mut(proj, cid, *lid)?;
            let fx = l.props.sub_mut("effects").ok_or_else(|| bad("effect.apply", "this layer type has no effects"))?;
            let same = fx.groups().filter(|g| g.match_id == spec.id).count();
            let name = if same == 0 { spec.name.to_string() } else { format!("{} {}", spec.name, same + 1) };
            let mut g = kition_motion_effects::instantiate(spec, &mut Ids(&mut next), &name, *size);
            if let Some(params) = &params {
                set_params(&mut g, params, spec.name).map_err(|e| bad("effect.apply", e))?;
            }
            out.push(g.uid);
            fx.children.push(g.into());
            proj.next_id = next;
            st.selected_props = vec![(*lid, *out.last().unwrap_or(&0))];
        }
        st.last_effect = Some(spec.id.to_string());
        Ok(out)
    })?;
    // Each instance's property path prefix (`effects/#2`), ready for prop.set paths.
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let paths: Vec<Value> = sizes
        .iter()
        .zip(&uids)
        .map(|((lid, _), uid)| {
            let n = comp.layer(*lid).and_then(|l| l.props.group("effects")).and_then(|fx| fx.children.iter().position(|c| c.uid() == *uid));
            json!(n.map(|n| format!("effects/#{}", n + 1)))
        })
        .collect();
    let lids: Vec<LayerId> = sizes.iter().map(|(lid, _)| *lid).collect();
    // The parameters of the first instance, so the next step can set or animate them by path.
    let listed = {
        let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
        match (lids.first(), paths.first().and_then(Value::as_str)) {
            (Some(lid), Some(prefix)) => comp.layer(*lid).and_then(|l| l.props.group(prefix)).map(|g| params_json(g, prefix)).unwrap_or_default(),
            _ => vec![],
        }
    };
    Ok(json!({"effects": uids, "paths": paths, "effect": spec.id, "params": listed}))
}

fn find_fx(s: &Session, p: &Value, cmd: &str) -> Result<(kition_motion_project::ItemId, kition_motion_project::LayerId, Uid)> {
    let (cid, lid) = super::layer_p(s, p, cmd)?;
    let layer = s.project.comp(cid).and_then(|c| c.layer(lid)).ok_or(EngineError::NoComp)?;
    let fx = layer.effects().ok_or_else(|| bad(cmd, "no effects"))?;
    let g = match p.get("effect") {
        Some(Value::Number(n)) => {
            let n = n.as_u64().unwrap_or(0);
            fx.groups().find(|g| g.uid == n).or_else(|| fx.groups().nth(n.saturating_sub(1) as usize))
        }
        Some(Value::String(name)) => fx.groups().find(|g| &g.name == name || g.match_id == *name),
        _ => s.state.selected_props.iter().find_map(|(l, u)| (*l == lid).then(|| fx.groups().find(|g| g.uid == *u)).flatten()),
    }
    .ok_or_else(|| bad(cmd, "no such effect"))?;
    Ok((cid, lid, g.uid))
}

/// `effect.warning`: what Effect Controls warns about an effect (a custom OCIO config it can't
/// read, or transforms it passes through), `null` when it renders as asked.
fn warning(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "effect.warning";
    let (cid, lid, uid) = find_fx(s, p, cmd)?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let layer = comp.layer(lid).ok_or(EngineError::NoComp)?;
    let g = layer.effects().and_then(|fx| fx.groups().find(|g| g.uid == uid)).ok_or_else(|| bad(cmd, "no such effect"))?;
    let t = super::time_p(s, p, Some(comp));
    let ctx = kition_motion_render::EvalCtx { project: &s.project, comp_id: cid, comp, time: t, expr: s.expr.as_deref(), footage: None };
    Ok(json!({"effect": uid, "warning": kition_motion_effects::warning(g, &mut |pr| ctx.value(layer, pr))}))
}

/// A colour parameter's eyedropper on a keyer (Effect Controls): the colour of the effect's input
/// (the layer's pixels before this effect, so a keyed screen can still be picked) at a
/// layer-space point, set on the parameter. `average` takes the 5 × 5 pixels around it.
fn pick_color(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "effect.pickColor";
    let (cid, lid, fx_uid) = find_fx(s, p, cmd)?;
    let (Some(x), Some(y)) = (f_p(p, "x"), f_p(p, "y")) else { return Err(bad(cmd, "missing `x` / `y` (layer pixels)")) };
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let layer = comp.layer(lid).ok_or(EngineError::NoComp)?;
    let fx = layer.effects().ok_or_else(|| bad(cmd, "no effects"))?;
    let index = fx.groups().position(|g| g.uid == fx_uid).ok_or_else(|| bad(cmd, "no such effect"))?;
    let g = fx.groups().nth(index).ok_or_else(|| bad(cmd, "no such effect"))?;
    let prop = match (p.get("prop").and_then(Value::as_u64), str_p(p, "param")) {
        (Some(uid), _) => g.find(uid),
        (None, Some(path)) => g.prop(path),
        _ => return Err(bad(cmd, "missing `param` (e.g. `screenColour`) or `prop` (uid)")),
    }
    .filter(|pr| pr.ui == kition_motion_project::ParamUi::Color)
    .ok_or_else(|| bad(cmd, "not a colour parameter of this effect"))?;
    let t = super::time_p(s, p, Some(comp));
    let ctx = kition_motion_render::EvalCtx { project: &s.project, comp_id: cid, comp, time: t, expr: s.expr.as_deref(), footage: None };
    let mut r = kition_motion_render::Renderer::new(&s.project, &*s.footage, kition_motion_render::RenderOpts::default());
    r.expr = s.expr.as_deref();
    r.cache = Some(&s.layer_cache);
    let buf = r.layer_input(&ctx, layer, index).ok_or_else(|| bad(cmd, "this layer has no pixels"))?;
    let (px, py) = buf.to_px([x, y]);
    let (cx, cy) = (px.floor() as i64, py.floor() as i64);
    let rad = if b_p(p, "average") == Some(true) { 2 } else { 0 };
    let mut sum = [0.0f32; 4];
    for yy in cy.saturating_sub(rad)..=cy.saturating_add(rad) {
        for xx in cx.saturating_sub(rad)..=cx.saturating_add(rad) {
            // Outside the layer reads as transparent.
            let c = buf.img.get(xx, yy);
            (0..4).for_each(|i| sum[i] += c[i]);
        }
    }
    if sum[3] <= 1e-6 {
        return Err(bad(cmd, "nothing to sample there (transparent or outside the layer)"));
    }
    // Straight colour of the (averaged) premultiplied pixels.
    let color: Vec<f64> = (0..3).map(|i| (sum[i] / sum[3]).clamp(0.0, 1.0) as f64).chain([1.0]).collect();
    let uid = prop.uid;
    s.execute("prop.set", json!({"comp": cid.0, "layer": lid.0, "prop": uid, "value": color}))?;
    Ok(json!({"prop": uid, "color": color}))
}

fn remove(s: &mut Session, p: &Value) -> Result<Value> {
    let cid = super::comp_id(s, p)?;
    let fx = effects_p(s, p, "effect.remove")?;
    let label = if fx.len() == 1 { "Remove Effect" } else { "Remove Effects" };
    s.edit(label, None, |proj, st| {
        for (lid, uid) in &fx {
            let l = layer_mut(proj, cid, *lid)?;
            if let Some(fx) = l.props.sub_mut("effects") {
                fx.children.retain(|c| c.uid() != *uid);
            }
            st.selected_props.retain(|(_, u)| u != uid);
        }
        Ok(())
    })?;
    Ok(json!(fx.len()))
}

fn remove_all(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, ids) = layers_p(s, p)?;
    s.edit("Remove All Effects", None, |proj, _| {
        for lid in &ids {
            if let Some(fx) = layer_mut(proj, cid, *lid)?.props.sub_mut("effects") {
                fx.children.clear();
            }
        }
        Ok(())
    })?;
    Ok(Value::Null)
}

fn toggle(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, lid, uid) = find_fx(s, p, "effect.toggle")?;
    let v = b_p(p, "value");
    let r = s.edit("Toggle Effect", None, |proj, _| {
        let l = layer_mut(proj, cid, lid)?;
        let g = l.props.find_group_mut(uid).ok_or_else(|| bad("effect.toggle", "gone"))?;
        g.enabled = v.unwrap_or(!g.enabled);
        Ok(g.enabled)
    })?;
    Ok(json!(r))
}

fn reorder(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, lid, uid) = find_fx(s, p, "effect.reorder")?;
    let to = p.get("index").and_then(Value::as_u64).ok_or_else(|| bad("effect.reorder", "missing `index` (1-based)"))? as usize;
    s.edit("Reorder Effect", None, |proj, _| {
        let fx = layer_mut(proj, cid, lid)?.props.sub_mut("effects").ok_or_else(|| bad("effect.reorder", "no effects"))?;
        let Some(i) = fx.children.iter().position(|c| c.uid() == uid) else { return Ok(()) };
        let g = fx.children.remove(i);
        let to = to.saturating_sub(1).min(fx.children.len());
        fx.children.insert(to, g);
        Ok(())
    })?;
    Ok(Value::Null)
}

fn duplicate(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, lid, uid) = find_fx(s, p, "effect.duplicate")?;
    let r = s.edit("Duplicate Effect", None, |proj, st| {
        let mut next = proj.next_id;
        let fx = layer_mut(proj, cid, lid)?.props.sub_mut("effects").ok_or_else(|| bad("effect.duplicate", "no effects"))?;
        let Some(i) = fx.children.iter().position(|c| c.uid() == uid) else { return Ok(0) };
        let mut g = fx.children[i].as_group().cloned().ok_or_else(|| bad("effect.duplicate", "not a group"))?;
        g.reassign_uids(&mut next);
        g.name = unique_name(fx, &g.name);
        let u = g.uid;
        fx.children.insert(i + 1, g.into());
        proj.next_id = next + 1;
        st.selected_props = vec![(lid, u)];
        Ok(u)
    })?;
    Ok(json!(r))
}

/// Effect groups selected in Effect Controls / the timeline: (layer, effect uid), in stack order.
pub(crate) fn selected_effects(s: &Session) -> Vec<(LayerId, Uid)> {
    let Some(c) = s.active_comp() else { return vec![] };
    let mut out = vec![];
    for l in &c.layers {
        let Some(fx) = l.effects() else { continue };
        for g in fx.groups() {
            if s.state.selected_props.iter().any(|(sl, u)| *sl == l.id && *u == g.uid) {
                out.push((l.id, g.uid));
            }
        }
    }
    out
}

/// Effects named by `effect` / `effects` (uids, names or 1-based indexes), else the selection.
fn effects_p(s: &Session, p: &Value, cmd: &str) -> Result<Vec<(LayerId, Uid)>> {
    if p.get("effect").is_some() {
        let (_, lid, uid) = find_fx(s, p, cmd)?;
        return Ok(vec![(lid, uid)]);
    }
    if let Some(Value::Array(a)) = p.get("effects") {
        let mut out = vec![];
        for v in a {
            let mut q = p.clone();
            if let Some(o) = q.as_object_mut() {
                o.remove("effects");
                o.insert("effect".into(), v.clone());
            }
            let (_, lid, uid) = find_fx(s, &q, cmd)?;
            out.push((lid, uid));
        }
        return Ok(out);
    }
    let sel = selected_effects(s);
    if sel.is_empty() {
        return Err(bad(cmd, "no effect selected"));
    }
    Ok(sel)
}

/// Edit ▸ Copy with effects selected: puts the effect instances on the effect clipboard.
fn copy(s: &mut Session, p: &Value) -> Result<Value> {
    let fx = effects_p(s, p, "effect.copy")?;
    let comp = if p.get("effect").is_some() || p.get("effects").is_some_and(Value::is_array) { s.project.comp(super::comp_id(s, p)?) } else { s.active_comp() }
        .ok_or(EngineError::NoComp)?;
    let groups: Vec<PropGroup> =
        fx.iter().filter_map(|(l, u)| comp.layer(*l).and_then(|l| l.effects()).and_then(|e| e.groups().find(|g| g.uid == *u)).cloned()).collect();
    s.state.clipboard.clear();
    s.state.key_clipboard.clear();
    s.state.contents_clipboard.clear();
    s.state.link_clipboard = None;
    s.state.clip_is_keys = false;
    let n = groups.len();
    s.state.effect_clipboard = groups;
    Ok(json!({"effects": n}))
}

/// A unique instance name among `fx`'s effects (`Gaussian Blur`, `Gaussian Blur 2`…).
pub(crate) fn unique_name(fx: &PropGroup, base: &str) -> String {
    if !fx.groups().any(|g| g.name == base) {
        return base.to_string();
    }
    let stem = base.rsplit_once(' ').filter(|(_, n)| n.parse::<u32>().is_ok()).map(|(s, _)| s).unwrap_or(base);
    (2..).map(|i| format!("{stem} {i}")).find(|n| !fx.groups().any(|g| &g.name == n)).unwrap_or_else(|| base.to_string())
}

/// Edit ▸ Paste with effects on the clipboard: adds copies to every target layer, after the
/// layer's selected effect when it has one, else at the end of its stack.
fn paste(s: &mut Session, p: &Value) -> Result<Value> {
    let clip = s.state.effect_clipboard.clone();
    if clip.is_empty() {
        return Err(bad("effect.paste", "no effects on the clipboard"));
    }
    let (cid, ids) = layers_p(s, p)?;
    if ids.is_empty() {
        return Err(bad("effect.paste", "select a layer to paste effects onto"));
    }
    let sel = selected_effects(s);
    let label = if clip.len() == 1 { format!("Paste {}", clip[0].name) } else { "Paste Effects".to_string() };
    let uids = s.edit(&label, None, |proj, st| {
        let mut out = vec![];
        let mut next = proj.next_id;
        let mut new_sel = vec![];
        for lid in &ids {
            let l = layer_mut(proj, cid, *lid)?;
            let Some(fx) = l.props.sub_mut("effects") else { continue };
            let at = sel
                .iter()
                .filter(|(sl, _)| sl == lid)
                .filter_map(|(_, u)| fx.children.iter().position(|c| c.uid() == *u))
                .max()
                .map(|i| i + 1)
                .unwrap_or(fx.children.len());
            for (k, g) in clip.iter().enumerate() {
                let mut g = g.clone();
                g.reassign_uids(&mut next);
                g.name = unique_name(fx, &g.name);
                out.push(g.uid);
                new_sel.push((*lid, g.uid));
                fx.children.insert(at + k, g.into());
            }
        }
        proj.next_id = next + 1;
        st.selected_props = new_sel;
        Ok(out)
    })?;
    Ok(json!({"effects": uids}))
}

/// Effect Controls' Reset: every parameter back to its default. Animated parameters get a
/// keyframe at the current time with the default value (as is usual); static ones
/// change their value.
fn reset(s: &mut Session, p: &Value) -> Result<Value> {
    let (cid, lid, uid) = find_fx(s, p, "effect.reset")?;
    let comp = s.project.comp(cid).ok_or(EngineError::NoComp)?;
    let layer = comp.layer(lid).ok_or(EngineError::NoComp)?;
    let size = kition_motion_render::effect_bounds(&s.project, comp, layer).0;
    let lt = layer.layer_time(s.time_of(cid));
    let g = layer.effects().and_then(|fx| fx.groups().find(|g| g.uid == uid)).ok_or_else(|| bad("effect.reset", "gone"))?;
    let spec = match &g.kind {
        GroupKind::Effect { effect } => kition_motion_effects::find(effect),
        _ => None,
    }
    .ok_or_else(|| bad("effect.reset", "unknown effect"))?;
    let name = g.name.clone();
    s.edit(&format!("Reset {name}"), None, |proj, _| {
        let g = layer_mut(proj, cid, lid)?.props.find_group_mut(uid).ok_or_else(|| bad("effect.reset", "gone"))?;
        for ps in &spec.params {
            // (`group/param` ids live in twirl-down groups.)
            if let Some(pr) = g.prop_mut(ps.id) {
                pr.set_value_at(lt, kition_motion_effects::default_value(ps, size));
            }
        }
        Ok(())
    })?;
    Ok(Value::Null)
}

fn last(s: &mut Session, p: &Value) -> Result<Value> {
    let id = s.state.last_effect.clone().ok_or_else(|| bad("effect.applyLast", "no effect applied yet"))?;
    let mut q = p.clone();
    if let Some(o) = q.as_object_mut() {
        o.insert("effect".into(), json!(id));
    } else {
        q = json!({"effect": id});
    }
    apply(s, &q)
}

fn list(s: &mut Session, p: &Value) -> Result<Value> {
    let filter = str_p(p, "filter").map(str::to_lowercase);
    // Point parameters are stored, read (`prop.get`) and written (`prop.set`) in layer pixels;
    // their definitions hold fractions of the layer's size. Listed against the active comp
    // (a comp-sized layer), the default is in those pixels; with no comp, it stays a fraction
    // and says so.
    let frame = s.active_comp().map(|c| [f64::from(c.width), f64::from(c.height)]);
    let v: Vec<Value> = kition_motion_effects::all()
        .into_iter()
        .filter(|e| {
            filter
                .as_ref()
                .is_none_or(|f| kition_motion_effects::name_matches(e, f) || e.id.contains(f.as_str()) || e.category.to_ascii_lowercase().contains(f))
        })
        .map(|e| {
            json!({"id": e.id, "name": e.name, "category": e.category, "gpu": e.gpu, "float": e.float, "params": e.params.iter().map(|p| {
                let mut out = json!({"id": p.id, "name": p.name, "type": p.default.kind_name(), "default": p.default.to_json()});
                if matches!(p.ui, kition_motion_project::ParamUi::Point) {
                    match frame {
                        Some(size) => {
                            out["default"] = kition_motion_effects::default_value(p, size).to_json();
                            out["unit"] = json!("px");
                        }
                        None => out["unit"] = json!("layer"),
                    }
                }
                match &p.ui {
                    kition_motion_project::ParamUi::Popup { options } => {
                        // Engine/MCP popup values are zero-based; JSX exposes one-based values.
                        out["options"] = json!(options.iter().enumerate().map(|(value, label)| json!({"value": value, "label": label})).collect::<Vec<_>>());
                    }
                    kition_motion_project::ParamUi::Slider { min, max, slider_min, slider_max, decimals } => {
                        out["min"] = json!(min);
                        out["max"] = json!(max);
                        out["sliderMin"] = json!(slider_min);
                        out["sliderMax"] = json!(slider_max);
                        out["decimals"] = json!(decimals);
                    }
                    _ => {}
                }
                out
            }).collect::<Vec<_>>()})
        })
        .collect();
    Ok(json!(v))
}

impl Session {
    /// The named channels of layer `lid` in the active comp at the current time: its
    /// multi-layer OpenEXR footage's layers and channels (`diffuse.R`, `depth.Z`, …), which
    /// EXtractoR and the 3D Channel effects read. `None` for other layers.
    pub fn layer_channels(&self, lid: LayerId) -> Option<std::sync::Arc<kition_motion_raster::AuxChannels>> {
        let layer = self.active_comp()?.layer(lid)?;
        let kition_motion_project::LayerSource::Footage { item } = layer.source else { return None };
        let kition_motion_project::ItemKind::Footage(f) = &self.project.item(item)?.kind else { return None };
        self.footage.aux(item, f, layer.layer_time(self.time()))
    }
}

/// `layer.channels`: the layers and channels of a layer's multi-layer OpenEXR footage, and
/// what each layer shows in red, green, blue and alpha (EXtractoR's popups).
fn channels(s: &mut Session, p: &Value) -> Result<Value> {
    let (_, lid) = super::layer_p(s, p, "layer.channels")?;
    let Some(aux) = s.layer_channels(lid) else { return Ok(json!({"channels": [], "layers": []})) };
    let layers: Vec<Value> = aux.layers().into_iter().map(|l| json!({"name": l, "rgba": aux.layer_rgba(l)})).collect();
    Ok(json!({"channels": aux.names(), "layers": layers}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!("effect.apply", "Apply Effect", [], None, "{effect: id|name (e.g. Gaussian Blur), layers?, params?: {setting: value}}", has_layers, apply),
        crate::query!(
            "layer.channels",
            "Layer Channels",
            "{layer?} → {channels: [name], layers: [{name, rgba: [r, g, b, a channel]}]} — a multi-layer OpenEXR layer's channels (EXtractoR's red / green / blue / alpha take these names)",
            channels
        ),
        cmd!(
            "effect.pickColor",
            "Pick Effect Colour",
            [],
            None,
            "{layer?, effect?: index|uid|name, param: id (e.g. screenColour) | prop: uid, x, y (effect space), average?: bool (5×5), time?} — the colour of the effect's input there",
            has_layers,
            pick_color
        ),
        cmd!("effect.applyLast", "Last Effect", ["Effect"], Some("Cmd+Alt+Shift+E"), "{layers?}", has_layers, last),
        cmd!("effect.removeAll", "Remove All", ["Effect"], Some("Cmd+Shift+E"), "{layers?}", has_layers, remove_all),
        cmd!("effect.remove", "Remove Effect", [], None, "{layer?, effect: index|uid|name}", has_layers, remove),
        cmd!("effect.toggle", "Toggle Effect", [], None, "{layer?, effect, value?}", has_layers, toggle),
        cmd!("effect.reorder", "Reorder Effect", [], None, "{layer?, effect, index}", has_layers, reorder),
        cmd!("effect.duplicate", "Duplicate Effect", [], None, "{layer?, effect}", has_layers, duplicate),
        cmd!("effect.copy", "Copy Effects", [], None, "{layer?, effect? | effects?: [uid|name|index]} (default: the selected effects)", has_layers, copy),
        cmd!("effect.paste", "Paste Effects", [], None, "{layers?} — adds the copied effects to the layers", has_layers, paste),
        cmd!("effect.reset", "Reset Effect", [], None, "{layer?, effect}", has_layers, reset),
        crate::query!(
            "effect.list",
            "List Effects",
            "{filter?} → [{params: [{id, name, type, default, unit?, options?, min?, max?, sliderMin?, sliderMax?, decimals?}]}]; popup indices are zero-based (JSX uses one-based values); point defaults (unit `px`) are pixels of a comp-sized layer of the active comp, as prop.set takes them (unit `layer`: fractions of the layer size, when no comp is open)",
            list
        ),
        crate::query!(
            "effect.warning",
            "Effect Warning",
            "{layer?, effect?: index|uid|name (default: the selected effect), time?} → {effect, warning: string|null} — what Effect Controls warns about (an OCIO config it can't read, transforms it passes through)",
            warning
        ),
        crate::query!(
            "effect.plugins.list",
            "List Effect Plug-ins",
            "{} → {api, wasm, plugins: [{id, name, category, version, author, source, params}]}",
            plugins_list
        ),
        cmd!(
            "effect.plugins.load",
            "Load Effect Plug-in...",
            [],
            None,
            "{path (.wasm / .wat plug-in, API v1) | folder (loads every .wasm in it)}",
            super::always,
            plugins_load
        ),
    ]
}

#[cfg(test)]
mod list_metadata_tests {
    use super::*;

    #[test]
    fn effect_list_exposes_popup_labels_and_numeric_slider_limits() {
        let mut s = Session::default();
        let menus = s.execute("effect.list", json!({"filter": "ec.control.dropdown"})).unwrap();
        assert_eq!(menus.as_array().map(Vec::len), Some(1));
        let menu = &menus[0]["params"][0];
        assert_eq!(menu["type"], "enum");
        assert_eq!(menu["default"], 0);
        assert_eq!(
            menu["options"],
            json!([
                {"value": 0, "label": "Item 1"},
                {"value": 1, "label": "Item 2"},
                {"value": 2, "label": "Item 3"}
            ])
        );
        let sliders = s.execute("effect.list", json!({"filter": "ec.control.slider"})).unwrap();
        assert_eq!(sliders.as_array().map(Vec::len), Some(1));
        let slider = &sliders[0]["params"][0];
        assert_eq!(slider["type"], "scalar");
        assert_eq!(slider["min"], -1_000_000.0);
        assert_eq!(slider["max"], 1_000_000.0);
        assert_eq!(slider["sliderMin"], 0.0);
        assert_eq!(slider["sliderMax"], 100.0);
        assert_eq!(slider["decimals"], 2);
        assert!(slider["options"].is_null());
    }
}

/// PROMO-10a (`regressions/issue-effect-list-point-units`): `effect.list` reports a point
/// parameter's default in the units `prop.get` and `prop.set` use.
#[cfg(test)]
mod list_point_unit_tests {
    use super::*;

    fn start_of_ramp(s: &mut Session) -> Value {
        let list = s.execute("effect.list", json!({"filter": "ec.generate.gradientramp"})).unwrap();
        list[0]["params"].as_array().unwrap().iter().find(|p| p["id"] == "start").unwrap().clone()
    }

    #[test]
    fn effect_list_point_defaults_are_pixels_of_the_active_comp() {
        let mut s = Session::default();
        s.execute("comp.new", json!({"name": "Ramp", "width": 1080, "height": 1920, "frameRate": 30, "duration": 2, "open": true})).unwrap();
        let listed = start_of_ramp(&mut s);
        assert_eq!(listed["unit"], "px");
        assert_eq!(listed["default"], json!([540.0, 0.0]));
        // Applied to a comp-sized layer, the parameter holds the listed default.
        let layer = s.execute("layer.newSolid", json!({"name": "Ramp", "color": "#000000"})).unwrap()["layer"].clone();
        s.execute("effect.apply", json!({"effect": "Gradient Ramp", "layers": [layer]})).unwrap();
        let got = s.execute("prop.get", json!({"layer": layer, "path": "effects/#1/start"})).unwrap();
        assert_eq!(got["value"], listed["default"]);
    }

    #[test]
    fn effect_list_point_defaults_without_a_comp_say_they_are_layer_fractions() {
        let mut s = Session::default();
        let listed = start_of_ramp(&mut s);
        assert_eq!(listed["unit"], "layer");
        assert_eq!(listed["default"], json!([0.5, 0.0]));
    }
}
