//! Explicit call context (task card C4-motion): the editor state a call reads besides its
//! parameters, set for one call and put back after it, so nothing an agent does depends on or
//! leaves behind the editor's active composition, current times or selections (kition-sdk's
//! `KitDocument::editor_state`, `set_context`, `restore_editor_state`; Kit Process Protocol section
//! 7a).
//!
//! None of this is an edit: the project, its revision, its dirty state and the undo history stay
//! as they are. The work area is not editor state here: it is the composition's, saved with it and
//! undone like any edit.

use std::collections::BTreeMap;

use kition_motion_project::{Comp, ItemId, LayerId, Uid};
use kition_motion_time::Tick;
use serde::{Deserialize, Serialize};

use crate::{EngineError, KeyRef, Result, Session, VertexRef};

/// The editor state a call may read and must not leave behind: which comp is active (and the open
/// comp tabs), each comp's current time, the selected layers and the properties, keyframes and mask
/// vertices selected on them, the Project panel's selection, the selected 3D camera track points
/// and current track, each comp's viewer 3D view, the region of interest and the text editing
/// session (target, range, caret and pending character style). Opaque to hosts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallState {
    active_comp: Option<ItemId>,
    open_comps: Vec<ItemId>,
    times: BTreeMap<ItemId, Tick>,
    selected_layers: Vec<LayerId>,
    selected_props: Vec<(LayerId, Uid)>,
    selected_keys: Vec<KeyRef>,
    selected_vertices: Vec<VertexRef>,
    project_selection: Vec<ItemId>,
    camera_points: Vec<u32>,
    current_track: Option<(LayerId, Uid)>,
    views3d: BTreeMap<ItemId, kition_motion_render::three_d::Views3D>,
    region_of_interest: Option<[f64; 4]>,
    text_edit: Option<crate::commands::text_edit::TextEdit>,
}

/// The current time for `seconds` of comp time in `comp`: the nearest tick, then the start of the
/// frame showing it, as `file.render` draws `target.time` (3.5 s at 29.97 fps is frame 104, which
/// starts at 3.4701 s); `None` outside 0 to before the comp's end. The one conversion of a call's
/// `at.time`.
pub fn time_at(comp: &Comp, seconds: f64) -> Option<Tick> {
    let t = Tick::from_seconds_f64(seconds);
    (seconds.is_finite() && t >= Tick::ZERO && t < comp.duration).then(|| comp.frame_rate.snap(t))
}

/// Seconds as an error says them: at most three decimals.
fn said(t: Tick) -> f64 {
    (t.seconds() * 1000.0).round() / 1000.0
}

/// A comp as an error names it: `7 "Main"`.
fn named(s: &Session, id: ItemId) -> String {
    format!("{} {:?}", id.0, s.project.item(id).map(|item| item.name.as_str()).unwrap_or(""))
}

/// The project's comps, as an error lists them.
fn comps(s: &Session) -> String {
    let all: Vec<String> = s.project.comps().map(|(id, _)| named(s, *id)).collect();
    if all.is_empty() { "it has none".into() } else { format!("its comps (id \"name\") are {}", all.join(", ")) }
}

/// One member of a call's `on`: a layer id, or a layer's name.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerRef {
    Id(u64),
    Name(String),
}

impl std::fmt::Display for LayerRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(id) => write!(f, "{id}"),
            Self::Name(name) => write!(f, "{name:?}"),
        }
    }
}

impl Session {
    /// The editor state ([`CallState`]) as it is now.
    pub fn call_state(&self) -> CallState {
        let st = &self.state;
        CallState {
            active_comp: st.active_comp,
            open_comps: st.open_comps.clone(),
            times: st.times.clone(),
            selected_layers: st.selected_layers.clone(),
            selected_props: st.selected_props.clone(),
            selected_keys: st.selected_keys.clone(),
            selected_vertices: st.selected_vertices.clone(),
            project_selection: st.project_selection.clone(),
            camera_points: st.camera_points.clone(),
            current_track: st.current_track,
            views3d: st.views3d.clone(),
            region_of_interest: st.region_of_interest,
            text_edit: st.text_edit.clone(),
        }
    }

    /// Puts back what [`Session::call_state`] answered, as far as the project still has it: a comp,
    /// layer or item the call deleted is left out (an active comp that is gone leaves the first
    /// open one active, as closing it does). No edit.
    pub fn restore_call_state(&mut self, saved: &CallState) {
        let saved = saved.clone();
        let st = &mut self.state;
        st.active_comp = saved.active_comp;
        st.open_comps = saved.open_comps;
        st.times = saved.times;
        st.selected_layers = saved.selected_layers;
        st.selected_props = saved.selected_props;
        st.selected_keys = saved.selected_keys;
        st.selected_vertices = saved.selected_vertices;
        st.project_selection = saved.project_selection;
        st.camera_points = saved.camera_points;
        st.current_track = saved.current_track;
        st.views3d = saved.views3d;
        st.region_of_interest = saved.region_of_interest;
        st.text_edit = saved.text_edit;
        let project = self.project.clone();
        self.state.times.retain(|id, _| project.comp(*id).is_some());
        self.state.views3d.retain(|id, _| project.comp(*id).is_some());
        self.sanitize_state();
    }

    /// Sets the editor state one call reads: `comp` (an item id) becomes the active comp, `seconds`
    /// of comp time its current time ([`time_at`]), and `on` the layer selection: exactly the
    /// layers it names, by id (or by a name only one layer of the comp has), with the properties,
    /// keyframes and mask vertices selected on other layers dropped. With no `comp`, the active one
    /// is meant. Everything is checked before anything changes: a comp or layer the project lacks,
    /// a stack position or `#n` where an id belongs, a name two layers share, a layer of another
    /// comp or a time outside the comp is an error that says what is valid, and nothing changes. No
    /// edit.
    pub fn set_call_context(&mut self, comp: Option<u64>, seconds: Option<f64>, on: &[LayerRef]) -> Result<()> {
        let error = |message: String| Err(EngineError::Other(message));
        if let Some(id) = comp
            && self.project.comp(ItemId(id)).is_none()
        {
            return error(format!("`at.part.comp` is {id}, which is no comp of this project: {}; give {{\"comp\": id}}", comps(self)));
        }
        let target = comp.map(ItemId).or(self.active_comp_id());
        let Some((target, c)) = target.and_then(|id| self.project.comp(id).map(|c| (id, c))) else {
            if seconds.is_some() || !on.is_empty() {
                return error(format!("no comp is active to set a time or layers in: name one in `at.part` ({{\"comp\": id}}); {}", comps(self)));
            }
            return Ok(());
        };
        let time = match seconds {
            Some(given) => match time_at(c, given) {
                Some(t) => Some(t),
                None => {
                    let last = c.frame_rate.snap(Tick((c.duration.0 - 1).max(0)));
                    return error(format!(
                        "`at.time` is {given} s, outside comp {}, which runs from 0 to {} s: give a time from 0 to {} s (where its last frame starts), in comp seconds",
                        named(self, target),
                        said(c.duration),
                        said(last)
                    ));
                }
            },
            None => None,
        };
        let mut layers = Vec::with_capacity(on.len());
        for member in on {
            layers.push(self.on_layer(target, c, member).map_err(EngineError::Other)?);
        }
        // Checked: now set.
        let st = &mut self.state;
        if st.active_comp != Some(target) {
            st.active_comp = Some(target);
            if !st.open_comps.contains(&target) {
                st.open_comps.push(target);
            }
            // As opening it does: what was selected was another comp's.
            st.selected_layers.clear();
            st.selected_props.clear();
            st.selected_keys.clear();
            st.selected_vertices.clear();
            st.current_track = None;
        }
        if let Some(t) = time {
            st.times.insert(target, t);
        }
        if !layers.is_empty() {
            st.selected_props.retain(|(layer, _)| layers.contains(layer));
            st.selected_keys.retain(|key| layers.contains(&key.layer));
            st.selected_vertices.retain(|vertex| layers.contains(&vertex.layer));
            st.selected_layers = layers;
        }
        Ok(())
    }

    /// The layer of comp `cid` that `member` of a call's `on` names, or why it names none.
    fn on_layer(&self, cid: ItemId, comp: &Comp, member: &LayerRef) -> std::result::Result<LayerId, String> {
        let listed = || {
            const SHOWN: usize = 20;
            let shown: Vec<String> = comp.layers.iter().take(SHOWN).map(|l| format!("{} {:?}", l.id.0, l.name)).collect();
            let more = comp.layers.len().saturating_sub(SHOWN);
            let more = if more > 0 { format!(" and {more} more") } else { String::new() };
            if shown.is_empty() { "it has no layers".to_owned() } else { format!("its layers, top first (id \"name\"): {}{more}", shown.join(", ")) }
        };
        let elsewhere = |id: LayerId| self.project.comps().find(|(other, c)| **other != cid && c.layer(id).is_some()).map(|(other, _)| *other);
        match member {
            LayerRef::Id(id) => {
                let id = LayerId(*id);
                if comp.layer(id).is_some() {
                    return Ok(id);
                }
                if let Some(other) = elsewhere(id) {
                    return Err(format!(
                        "`on` names layer {}, which is in comp {}, not in comp {} the call acts in: name its comp in `at.part` ({{\"comp\": {}}})",
                        id.0,
                        named(self, other),
                        named(self, cid),
                        other.0
                    ));
                }
                let position = usize::try_from(id.0).ok().filter(|n| (1..=comp.layers.len()).contains(n));
                let why = if position.is_some() { ", and a stack position is no id here" } else { "" };
                Err(format!(
                    "`on` names {}, which is no layer id of comp {}{why}: give layer ids as motion_inspect lists them; {}",
                    id.0,
                    named(self, cid),
                    listed()
                ))
            }
            // An id as receipts print it (`"51"`): the layer of that id, in this comp or another.
            LayerRef::Name(name) if name.parse::<u64>().is_ok_and(|id| comp.layer(LayerId(id)).is_some() || elsewhere(LayerId(id)).is_some()) => {
                self.on_layer(cid, comp, &LayerRef::Id(name.parse::<u64>().unwrap_or_default()))
            }
            LayerRef::Name(name) if name.starts_with('#') => Err(format!(
                "`on` names {name:?}, a stack position: `on` takes layer ids as motion_inspect lists them; {} of comp {}",
                listed(),
                named(self, cid)
            )),
            LayerRef::Name(name) => {
                let found: Vec<LayerId> = comp.layers.iter().filter(|l| &l.name == name).map(|l| l.id).collect();
                match found[..] {
                    [one] => Ok(one),
                    [] => Err(format!(
                        "`on` names {name:?}, which is no layer of comp {}: give layer ids as motion_inspect lists them; {}",
                        named(self, cid),
                        listed()
                    )),
                    _ => Err(format!(
                        "`on` names {name:?}, which {} layers of comp {} are called ({}): give layer ids",
                        found.len(),
                        named(self, cid),
                        found.iter().map(|id| id.0.to_string()).collect::<Vec<_>>().join(", ")
                    )),
                }
            }
        }
    }
}
