//! Editor-context comparison only. This does not certify object, field or deferred-write premises.
use super::super::{Row, families};
use kition_kit::{EditorState, KitDocument};
use kition_motion_kit::MotionDocument;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const PART: &[EditorState] = &[EditorState::Part];
const SELECTION: &[EditorState] = &[EditorState::Selection];
const TIME: &[EditorState] = &[EditorState::Time];
const PART_SELECTION: &[EditorState] = &[EditorState::Part, EditorState::Selection];
const PART_TIME: &[EditorState] = &[EditorState::Part, EditorState::Time];
const PART_SELECTION_TIME: &[EditorState] = &[EditorState::Part, EditorState::Selection, EditorState::Time];
const MODE: &[EditorState] = &[EditorState::Mode];
const AREA: &[EditorState] = &[EditorState::Area];

struct Mapping {
    bits: &'static [EditorState],
    complete: bool,
    kind: &'static str,
    note: &'static str,
}

fn context(bits: &'static [EditorState]) -> Mapping {
    Mapping {
        bits,
        complete: true,
        kind: "context",
        note: "Literal phrase names these editor-context inputs; conditions remain conditions, not unconditional reads.",
    }
}

fn non_context(note: &'static str) -> Mapping {
    Mapping { bits: &[], complete: true, kind: "non-context", note }
}

fn write_only() -> Mapping {
    Mapping { bits: &[], complete: true, kind: "write-only", note: "The phrase records outputs, not an input to the result." }
}

fn unresolved(bits: &'static [EditorState], note: &'static str) -> Mapping {
    Mapping { bits, complete: false, kind: "unresolved", note }
}

// These are whole-phrase decisions. In particular project_selection is never rewritten as
// Selection, a stored work area is never Area, and a write/unused time is never a Time input.
fn classify(row: &Row, phrase: &str) -> Mapping {
    if matches!(row.command, "edit.purge" | "edit.purgeUndo" | "edit.undo" | "edit.redo" | "edit.history.goto") {
        match phrase {
            "active_comp fallback" | "selected_layers/selected_props/selected_keys/selected_vertices" | "text_edit and clipboards choose dispatch" => {
                return unresolved(
                    &[],
                    "edit.rs:60-75,721-755,797-801,881-904: the shared SELECT phrase exceeds these handlers' direct inputs; history restoration and state sanitation need a separate closure proof.",
                );
            }
            _ => {}
        }
    }
    match phrase {
        "part"
        | "active comp"
        | "active_comp"
        | "active_comp always"
        | "active_comp captured before import"
        | "active_comp comparison"
        | "active_comp default"
        | "active_comp fallback"
        | "active_comp fallback and enablement"
        | "active_comp fallback/enablement"
        | "active_comp for defaults"
        | "active_comp for fallback defaults"
        | "active_comp for solid fps fallback"
        | "active_comp if comp omitted"
        | "active_comp if omitted"
        | "active_comp if omitted; CTI fetched but last-frame context substitutes out-frame"
        | "active_comp if omitted; active_comp comparison affects default start"
        | "active_comp if omitted; emits timeline reveal/toast"
        | "active_comp if omitted; time fetched but unused"
        | "active_comp if omitted; writes template_props_primary and bumps revision"
        | "active_comp if omitted; writes template_props_primary, bump and frontend panel event"
        | "active_comp if omitted; writes project_selection and frontend panel"
        | "active_comp only if comp omitted"
        | "active_comp only if comp omitted; time fetched but unused"
        | "active_comp only when comp omitted"
        | "active_comp solid fps fallback"
        | "active_comp when comp omitted"
        | "captured active_comp"
        | "part for environment fallback" => context(PART),

        "active comp/layer selection"
        | "active_comp always for selected_effects"
        | "active_comp and selected_layers fallbacks"
        | "active_comp if omitted; selected_layers insertion default; writes selection"
        | "active_comp target fallback; selected_layers insertion default; writes project_selection=new comp and inserted layer selection"
        | "active_comp/selected_layers defaults"
        | "active_comp/selected_layers defaults; initializes template_props_primary if absent"
        | "active_comp/selected_layers defaults; no CTI"
        | "active_comp/selected_layers for property defaults; selected_props when no explicit property; writes template_props_primary only if absent"
        | "active_comp/selected_layers if omitted; emits OpenUrl"
        | "active_comp/selected_layers if omitted; may open source comp or emit frontend event"
        | "active_comp/selected_layers if omitted; writes project_selection and frontend panel"
        | "active_comp/selected_layers if omitted; writes selected_props tracker; running-job state"
        | "active_comp/selected_layers only for omitted target or enabled() gate"
        | "active_comp/selected_layers when omitted"
        | "active_comp/selected_layers when omitted; selection order selects SOURCE"
        | "active_comp/selected_layers when omitted; writes project_selection=[source.item]"
        | "active_comp/selected_layers when omitted; writes selected_props and last_effect"
        | "active_comp/selected_layers/selected_props for omitted address; pending/running jobs"
        | "active_comp/selected_layers/selected_props for omitted addresses"
        | "active_comp/selected_layers/selected_props if omitted"
        | "active_comp/selected_layers/selected_props if omitted; writes selected_props to new group"
        | "active_comp/selected_layers/selected_props when omitted"
        | "part/selection defaults" => context(PART_SELECTION),

        "selection/open comp" => unresolved(
            &[],
            "app_more.rs:456-561 creates fresh destination comps before nested layer operations, then opens output; input selection versus completion output is not established by this phrase.",
        ),

        "active comp current time when time omitted" | "part and time for reference default" => context(PART_TIME),

        "active_comp if omitted; CTI always; selected_layers insertion default for absent master; writes selection during insertion and project_selection=output"
        | "active_comp if omitted; selected_layers for camera fallback; CTI always; writes selected_layers=[null]"
        | "active_comp if omitted; selected_layers light default; CTI always; writes selected_layers=[new backdrop]"
        | "active_comp/selected_layers defaults and insertion; CTI always; writes inserted selection"
        | "active_comp/selected_layers defaults; CTI always; general.default_spatial_linear"
        | "active_comp/selected_layers defaults; CTI for camera fallback; light frame evaluation also occurs but on=true result does not use it"
        | "active_comp/selected_layers when omitted; CTI when no time/frame"
        | "active_comp/selected_layers when omitted; selected_layers insertion gap; CTI always; writes layer selection as inserted"
        | "downstream current time; active_comp and selected_layers if omitted" => context(PART_SELECTION_TIME),

        "same active_comp/selected_layers/time defaults as layer.new3dPrimitive" => Mapping {
            note: "model3d.rs:228-250,423: identical new_primitive handler; all destination properties are newly constructed static values, so fetched time does not affect the result.",
            ..context(PART_SELECTION)
        },

        "first selected_layers if layer omitted"
        | "first selected_layers if omitted"
        | "selected_keys"
        | "selected_keys for target/default times"
        | "selected_layers"
        | "selected_layers and selected_props when omitted"
        | "selected_layers default"
        | "selected_layers defaults"
        | "selected_layers determines insertion"
        | "selected_layers fallback"
        | "selected_layers fallback and enablement"
        | "selected_layers for default insertion"
        | "selected_layers for omitted layer; selected_props if no explicit instances"
        | "selected_layers for omitted refs or enablement"
        | "selected_layers if index omitted"
        | "selected_layers if layer omitted"
        | "selected_layers if layers omitted"
        | "selected_layers if layers/layer omitted"
        | "selected_layers if omitted"
        | "selected_layers if omitted; CTI fetched but unused"
        | "selected_layers if omitted; filters selected_props only exact removed group UIDs"
        | "selected_layers if omitted; replaces selected_props with new group IDs"
        | "selected_layers if omitted; writes selection to first generated shape"
        | "selected_layers if omitted; writes selection to generated layer"
        | "selected_layers if omitted; writes selection to generated shape"
        | "selected_layers if omitted; writes selection to replacement"
        | "selected_layers when add/toggle"
        | "selected_layers when camera omitted or enablement needed"
        | "selected_layers when layer omitted"
        | "selected_layers when omitted"
        | "selected_layers when toLayer omitted"
        | "selected_layers/props"
        | "selected_layers/props/vertices for omitted targets"
        | "selected_layers/selected_props if omitted"
        | "selected_layers/selected_props if omitted; CTI fetched but unused"
        | "selected_layers/selected_props if omitted; filters selected_props only exact removed group UIDs (descendant selections remain)"
        | "selected_layers/selected_props/selected_keys/selected_vertices"
        | "selected_props"
        | "selected_props and selected_layers if layer/effect omitted"
        | "selected_props for omitted pin references"
        | "selected_props if animator omitted"
        | "selected_props if style absent"
        | "selected_props when mask omitted (can override explicit layer list)"
        | "selected_props when no path/prop"
        | "selected_props when omitted"
        | "selected_props when prop omitted"
        | "selected_props/layers"
        | "selected_vertices default"
        | "selected_vertices when add"
        | "selection"
        | "selection defaults"
        | "selection determines defaults-vs-document branch"
        | "selection determines insertion gap"
        | "selection fallback"
        | "selection for default targets"
        | "selection for enablement/default layer"
        | "selection for target"
        | "selection if explicit layers absent"
        | "selection if explicit targets absent"
        | "selection if layer absent"
        | "selection if layer absent and for can_track"
        | "selection if layers absent"
        | "selection if targets absent"
        | "selection insertion gap" => context(SELECTION),

        "selection for selected variant when layers omitted" => match row.command {
            "view.lookAtSelected" => context(SELECTION),
            "view.lookAtAll" => non_context("three_d.rs:564-569: all=true takes every eligible layer and does not read layer selection."),
            _ => unresolved(&[], "The selected-variant condition has not been established for this handler."),
        },

        "selected_vertices" => match row.command {
            "mask.deleteVertices" => context(SELECTION),
            _ => unresolved(&[], "Unqualified vertex state may be input selection or output cleanup; no read/write equivalence assumed."),
        },

        "selected_layers only for response UI target substitution" | "selected_layers only response UI target" => {
            unresolved(&[], "A response-only substitution is recorded; its necessity to the write result has not been adjudicated.")
        }
        "camera_points replace/add/toggle; revision bump" => {
            unresolved(&[], "camera_cmds.rs:294-318 reads prior camera-point selection for add/toggle; these IDs are not the layer selection that on sets.")
        }

        "selected_layers and active_comp only where has_layers enablement required" => match row.command {
            "layer.style.options" | "layer.openLayer" => context(PART_SELECTION),
            "effect.manage" | "anim.browsePresets" | "project.deleteWithoutConfirmation" => non_context(
                "frontend.rs:19,106-119: these three variants are always enabled and forward an event; the shared phrase's has_layers condition is false.",
            ),
            _ => unresolved(&[], "The conditional enablement variant has not been classified."),
        },

        "current time"
        | "current time default range"
        | "current time for animated path"
        | "current time for coercion sample and default destination"
        | "current time for sampling"
        | "current time if animated"
        | "current time only when time omitted"
        | "current time when font omitted"
        | "current time when start omitted"
        | "current time when time omitted"
        | "time"
        | "time (s.time active comp even if explicit comp)"
        | "time always for delegated assignment; also sample if time/frame omitted"
        | "time consumed by nested prop.set per face"
        | "time currentFrame"
        | "time fallback within layer.addItem"
        | "time for reference default"
        | "time if absent"
        | "time if absent and creation-at-start disabled and target active"
        | "time if explicit time absent"
        | "time if frame absent"
        | "time if frame absent (frame parameter is seconds)"
        | "time if not supplied"
        | "time if start absent"
        | "time if start absent (even static branch reads start)"
        | "time if time absent"
        | "time if time/frame absent"
        | "time only hold=current"
        | "time only if omitted"
        | "time only if target active"
        | "time only without time/frame"
        | "time unless time/frame supplied"
        | "time when start absent"
        | "time() of ACTIVE comp even with explicit target comp"
        | "time_of requested comp"
        | "time_of resolved comp"
        | "time_of resolved comp if at omitted"
        | "time_of selected main comp"
        | "time_of(comp)"
        | "time_of(comp) default"
        | "time_of(comp) for comp-space transform"
        | "time_of(comp) if no landmark keys"
        | "time_of(comp) when start omitted"
        | "time_of(comp) when time omitted" => context(TIME),

        "current time for set_group/fillStroke animated writes" => match row.command {
            "shape.dashes.add" | "shape.dashes.remove" => non_context(
                "shape_stroke.rs:71-129: dashes handlers directly edit base values, keys or children and never call the time-dependent set_group/fillStroke path.",
            ),
            _ => context(TIME),
        },

        "current time only if sampled keys (static branch none)" => {
            non_context("This frozen path is the static branch: its empty key list makes the fetched sample time irrelevant.")
        }
        "time fetched but unused for static vector conversion" => non_context("The frozen phrase explicitly states the fetched time is unused."),
        "time_p reads CTI if no time/frame, comp.frame_rate for frame (new properties static, time does not affect value)" => {
            non_context("The frozen path creates static properties; the phrase explicitly rules out a time-dependent result.")
        }
        "active_comp only when comp omitted; time fetched but not used unless set=begin/end" => {
            unresolved(PART, "Part is explicit; time eligibility depends on a set branch that needs an executable path fixture.")
        }
        "time or area by span" => context(TIME),

        "region_of_interest" | "region_of_interest fallback" => context(AREA),
        "region_of_interest only crop.useRoi=true without crop.roi" => match row.command {
            "renderQueue.setOutputModule" | "render.addOutputModule" => context(AREA),
            "renderQueue.saveTemplate" if row.path == "kind=outputModule; item/module or from or Movie default" => context(AREA),
            "renderQueue.remove"
            | "renderQueue.setRender"
            | "renderQueue.setRenderSettings"
            | "renderQueue.setOutput"
            | "renderQueue.move"
            | "renderQueue.duplicate"
            | "renderQueue.saveTemplate"
            | "renderQueue.applyTemplate"
            | "renderQueue.setLog" => non_context(
                "render_queue.rs:550-580,626-665,854-876,926-969: these handlers never give editor ROI to apply_output; the shared STATE phrase's crop condition cannot occur.",
            ),
            _ => unresolved(&[], "The conditional ROI helper path has not been proven for this handler."),
        },
        "area (workArea stored comp state)" | "work_area" => {
            non_context("engine/context.rs:8-10: work area is stored composition state, not EditorState::Area.")
        }

        "views3d.current" | "views3d.last" => context(MODE),
        "views3d.current/cams" => {
            unresolved(MODE, "The current view selects the operation mode; the custom view-camera data has no established exact EditorState mapping.")
        }
        "text_edit absent" => context(MODE),
        "text_edit" | "text_edit; writes pending insertion style in addition to normal edit branch" => {
            unresolved(MODE, "The text-editing mode is an input; caret/range/style state within it has not been mapped to an exact context bit.")
        }
        "text_edit and clipboards choose dispatch" => match row.command {
            "edit.clear" | "edit.cut" | "edit.paste" | "edit.pasteTextFormattingOnly" | "edit.pasteTextMatchFormatting" => unresolved(
                MODE,
                "edit.rs:159,488,508; text_edit.rs:389,421: these handlers branch on text-edit mode; caret/range/style and clipboard closure remain unresolved.",
            ),
            _ => unresolved(
                &[],
                "edit.rs:118-156,581-636,638-703,777-794: the shared SELECT phrase is not a mode read of these direct handlers; delegated command/eligibility closures need a separate proof.",
            ),
        },
        "selected_layers/text_edit when font omitted" => unresolved(
            &[EditorState::Selection, EditorState::Mode],
            "text_edit.rs:476-480 uses the editing target and range to choose the implicit font. Selection and Mode are inputs; the exact caret/range/style representation remains unresolved.",
        ),

        "template_props_primary/active_comp defaults"
        | "template_props_primary/active_comp defaults; initializes primary if absent"
        | "template_props_primary/active_comp fallback; sets template_props_primary only if absent" => unresolved(
            PART,
            "Active composition is explicit; Template Properties primary composition is a distinct active destination and remains separately unresolved.",
        ),
        "template_props_primary/active_comp defaults; selected_props if target omitted" => {
            unresolved(PART_SELECTION, "Part and property selection are explicit; Template Properties primary destination remains unresolved.")
        }
        "template_props_primary/active_comp defaults; render host state closure unresolved" => {
            unresolved(PART, "The phrase explicitly leaves the rendering closure unresolved; Template Properties primary also needs a distinct mapping.")
        }
        "active_comp/project_selection fallbacks" | "project_selection and active_comp only if no comp/item" => {
            unresolved(PART, "Project-panel item selection is not the layer selection that on sets; its exact EditorState mapping is unresolved.")
        }
        "project_selection if item omitted; selected_layers for omitted index; writes layer selection" => {
            unresolved(SELECTION, "Layer-selection insertion is explicit; Project-panel selection remains distinct and unresolved.")
        }
        "project_selection; else active_comp/selected_layers if no numeric targets" => {
            unresolved(PART_SELECTION, "Part/layer selection are explicit; Project-panel selection remains distinct and unresolved.")
        }

        "part/current_track/selected_layers for eligibility" => {
            unresolved(PART_SELECTION, "Current tracker is a separate active object; its EditorState mapping is not assumed.")
        }
        "current_track and selection defaults" | "current_track/selection defaults" => {
            unresolved(SELECTION, "Current tracker is not the layer selection; it remains unresolved.")
        }
        "active_comp/selected_props/selected_layers for ct_p; CTI always selects solve frame; camera_points when points omitted"
        | "ct_p defaults; camera_points when omitted; CTI always" => unresolved(
            PART_SELECTION_TIME,
            "ct_p resolves composition and layer/property selection; camera-point selection is a distinct unresolved selection.",
        ),
        "ct_p defaults; CTI; writes camera selection and clears selected_props" => context(PART_SELECTION_TIME),
        "ct_p defaults; camera_points when points omitted; removes deleted IDs from camera_points; running jobs" => {
            unresolved(PART_SELECTION, "ct_p address defaults are explicit; camera-point selection remains unresolved.")
        }
        "running/pending camera jobs; status uses CTI and selected camera points" => {
            unresolved(TIME, "CTI is explicit; camera-point selection remains unresolved.")
        }
        "same panel/selection/part/time fallbacks as overlay" => unresolved(
            PART_TIME,
            "footage_panel.rs:26-36,139-167: selection means Project-panel items, not layers; footage-panel data remains distinct and unresolved.",
        ),
        "active_comp/open_comps; removes open comp; may change active_comp and clear selected_layers/props/keys" => {
            unresolved(PART, "Active composition is read; open tabs and state sanitation require a separate input/output proof.")
        }
        "file.import editor defaults; active_comp and selected_layers when omitted" => {
            unresolved(PART_SELECTION, "Part and layer selection are explicit; delegated import defaults remain unresolved.")
        }
        "same address defaults; CTI always determines released placement" => {
            unresolved(TIME, "The phrase explicitly names CTI; inherited address defaults need a separate closure proof.")
        }
        "items_p fallback project/layer selection; writes interpretation clipboard" => {
            unresolved(SELECTION, "Layer selection is explicit; Project-panel selection remains distinct and unresolved.")
        }

        "project_selection"
        | "project_selection fallback"
        | "project_selection fallback/enablement"
        | "project_selection for add; writes project_selection"
        | "project_selection if item omitted"
        | "project_selection if no comp/item"
        | "project_selection if omitted"
        | "project_selection if omitted; writes selection to new item IDs"
        | "project_selection source fallback"
        | "project_selection when items/item omitted"
        | "project_selection when omitted" => {
            unresolved(&[], "Project-panel item selection is not the layer selection that on sets; no exact EditorState equivalence is assumed.")
        }

        "Apply closure editor state"
        | "all editor state any delegated command reads/writes"
        | "arbitrary script-visible editor state"
        | "arbitrary script-visible state; console context if requested"
        | "callback editor-state dependencies"
        | "callback state inputs"
        | "delegated import selection/default comp state"
        | "each Apply closure editor state"
        | "nested command reads"
        | "nested command time/defaults"
        | "same defaults and completion selection as generate"
        | "single branch state"
        | "union of nested command editor-state inputs"
        | "union of nested command editor-state reads" => unresolved(
            &[],
            "This phrase names a dependency closure rather than enumerating its editor-state inputs; a fixture-specific closure proof is required.",
        ),

        "editor state sanitized"
        | "editor state snapshot"
        | "filters project_selection and sanitize_state removes invalid comp/layer/prop/key selections"
        | "filters selected_props matching removed UID"
        | "sanitize_state after removal"
        | "sanitize_state filters invalid editor references"
        | "sanitize_state filters removed editor references" => {
            unresolved(&[], "Snapshot/sanitation can preserve or filter existing editor data; read versus output cleanup has not been proven per path.")
        }

        "clone aligned offset initialized only if session paint.aligned and no existing offset"
        | "content_fill settings"
        | "content_fill settings retain unspecified fields"
        | "current_track"
        | "template_props_solo; writes template_props_solo and bumps revision"
        | "footage_panel"
        | "footage_panel source/range fallback"
        | "hide_locked_masks"
        | "items_p fallback selection"
        | "items_p fallback selection; emits OpenUrl"
        | "items_p fallback state"
        | "mask_interp"
        | "mask_track_method default"
        | "non-active-camera branch reads/writes views3d.cams only"
        | "open_comps"
        | "paint all unspecified stroke options: color/tip/channels/opacity/flow/mode/dynamics/erase/duration/clone source/position/time"
        | "paint existing options retained for omitted fields"
        | "paint options"
        | "puppet density/expansion/triangles defaults unless supplied"
        | "puppet options"
        | "puppet recording defaults"
        | "rotoscope brush defaults"
        | "roto existing options retain unspecified fields"
        | "script_windows"
        | "script_windows windows/dispatch"
        | "session.learn"
        | "shape_tool"
        | "shape_tool fill/stroke defaults"
        | "views3d.cams existing/default" => unresolved(
            &[],
            "This named session input is not assumed equivalent to a broad EditorState bit; its exact representation and result-dependent fields remain unresolved.",
        ),

        "all editor state reset"
        | "active_comp/time changes during opening new comps"
        | "cancels job/offloaded flag; no project assignment directly"
        | "clears current_track when deleted tracker was current"
        | "clears region_of_interest"
        | "custom-view branch writes views3d.cams; 2D branch frontend event only"
        | "editor reset"
        | "editor state reset"
        | "file.open editor reset"
        | "frontend event"
        | "replaces project editor state/history through replace_project"
        | "selection/open comp overwritten"
        | "selects new layer"
        | "selects new layer and clears selected_props/selected_keys"
        | "selects new layer; clears selected_props/selected_keys"
        | "selects replacement layer; optionally opens new comp"
        | "selects target; optionally opens new comp"
        | "writes CTI for active job comp"
        | "writes cancel flag; worker cancel discards replies"
        | "writes cancel flags"
        | "writes current_track"
        | "writes current_track and CTI last analyzed frame"
        | "writes current_track and selected_layers"
        | "writes effect_clipboard; clears other clipboards/link_clipboard; clip_is_keys=false"
        | "writes frontend window.panel contentAwareFill event"
        | "writes open_comps, active_comp and selection through open_comp"
        | "writes open_comps; if active, writes active_comp and clears selected_layers/selected_keys/selected_props"
        | "writes paint.clone_source/clone_point/clone_offset"
        | "writes project_selection"
        | "writes project_selection only what=footage; toasts all branches"
        | "writes project_selection to new folder"
        | "writes project_selection; opens new comp unless open=false"
        | "writes puppet defaults and show_mesh"
        | "writes running/progress/cache state"
        | "writes selected_keys clear"
        | "writes selected_layers"
        | "writes selected_layers and ring insert clears selected_props/keys"
        | "writes selected_layers created objects else camera; clears selected_props"
        | "writes selected_layers incoming"
        | "writes selected_layers on completion"
        | "writes selected_layers unless effect already selected"
        | "writes selected_layers/props/keys"
        | "writes selected_layers; replacement clears selected_props/selected_keys and filters selected_vertices"
        | "writes selected_props"
        | "writes selected_props and last_effect"
        | "writes selected_props clear"
        | "writes selected_props removing deleted pins"
        | "writes selected_props removing group/child selections"
        | "writes selected_props/keys and time=start"
        | "writes selected_props/layers and selected_keys clear for replace"
        | "writes selections and opens each new comp"
        | "writes show_mesh and/or density/expansion/triangles defaults"
        | "writes supplied diameter/refine_diameter/view/overlay_color/opacity/boundary_color/auto_propagate; clamps numeric values"
        | "writes supplied eye/poi/zoom"
        | "writes supplied method/range/alpha_expansion/lighting_correction/reference_layer/reference_time"
        | "writes supplied paint options; manual tip edits clear preset; reset overwrites defaults"
        | "writes supplied record_speed/smoothing/draft/show_mesh"
        | "writes text_edit if edit=true"
        | "writes tip diameter/angle/roundness/hardness/spacing/preset when supplied"
        | "writes views3d current/last"
        | "writes views3d current/last and selected_layers/props/keys"
        | "writes views3d.cams removes current view" => write_only(),

        "Roto jobs"
        | "Roto jobs/pending"
        | "Roto jobs/pending/cache"
        | "auto_save and export/nested-switch preferences"
        | "background task state only"
        | "browser home"
        | "browser host"
        | "captured import preferences"
        | "check_footage_on_open"
        | "comp/layer default prefs"
        | "config"
        | "config availability"
        | "config directory"
        | "config store"
        | "default_spatial_linear"
        | "effect_clipboard"
        | "every edit; no CTI"
        | "expr host/footage provider"
        | "expr_check"
        | "font registry"
        | "footage provider and expr host"
        | "grouped undo"
        | "history undo/redo"
        | "history.merge_key"
        | "history.merge_key/undo"
        | "history.merge_key/undo base"
        | "history.undo last project"
        | "host directory existence"
        | "import preferences"
        | "interpretation"
        | "job/offload state"
        | "jobs registry"
        | "jobs state; preserves dirty flag"
        | "key_clipboard"
        | "key_move"
        | "last_effect"
        | "layer insertion preferences"
        | "media_browser.folder/favorites"
        | "model preferences/status"
        | "model prefs/status"
        | "model status"
        | "models_dir/config"
        | "models_dir/config (none in kit)"
        | "nested switches preference"
        | "nested_switches and realtime_shadows preferences"
        | "new camera preferences"
        | "new layer/comp defaults from prefs"
        | "new null preferences"
        | "no editor state dependency; task may complete after outer command; apply has own stale-footage check"
        | "offload job state"
        | "plugin loader service availability"
        | "poll_models"
        | "prefs.appearance.cycle_mask_colors"
        | "prefs.general.default_spatial_linear"
        | "prefs.general.expression_pick_whip_compact"
        | "prefs.general.preserve_constant_vertex_count"
        | "preserve_constant_vertex_count for counted edits"
        | "prior footage_stamps; preserves dirty flag"
        | "project template preference"
        | "recent_footage"
        | "recent_presets"
        | "recent_projects"
        | "recent_projects preferences"
        | "render job"
        | "render job running state for enablement"
        | "render job state"
        | "render jobs"
        | "render preferences outside kit"
        | "runs after EVERY successful edit closure, before project installation; no CTI"
        | "script_settings/config"
        | "scripting permission prefs outside kit"
        | "sequence fps"
        | "sequence_rate preference"
        | "session path"
        | "session.path"
        | "session.path fallback"
        | "session.path/default output folder"
        | "split-above preference"
        | "text_clipboard"
        | "track and mask-track jobs"
        | "tracking job"
        | "tracking running eligibility"
        | "tracking/mask-track/warp jobs"
        | "warp running/pending job state"
        | "warp_job"
        | "warp_job cancel state"
        | "watch_folder fallback and enablement"
        | "watch_folder outside kit" => non_context(
            "Resource, job, history, clipboard or application preference state; it is not a composition, layer selection, playhead, editing mode, active editor destination or selected area.",
        ),

        _ => unresolved(&[], "No exact whole-phrase mapping established. Unknown is not a match."),
    }
}

#[derive(Default)]
struct Command {
    required: BTreeSet<String>,
    declared: BTreeSet<String>,
    unresolved: BTreeSet<String>,
    paths: Vec<String>,
    pinned: bool,
    advertised: bool,
}

pub(super) fn audit(document: &MotionDocument) -> Value {
    let declarations: BTreeMap<_, _> = document.commands().into_iter().map(|command| (command.id.clone(), command)).collect();
    let mut commands = BTreeMap::<&str, Command>::new();
    let mut paths = Vec::new();
    let mut phrases = BTreeMap::<&str, (usize, usize)>::new();
    for (family, rows) in families() {
        for (index, row) in rows.iter().enumerate() {
            let id = format!("{family}/{index}:{}:{}", row.command, row.path);
            let command = commands.entry(row.command).or_default();
            if let Some(declaration) = declarations.get(row.command) {
                command.declared = declaration.reads.iter().map(|state| state.as_str().to_owned()).collect();
                command.advertised = declaration.mutates;
            }
            command.pinned |= family == "pinned_r2_animation" || family == "pinned_r2_render_queue";
            command.paths.push(id.clone());
            let mut required = BTreeSet::new();
            let mut unknown = Vec::new();
            let mut mappings = Vec::new();
            for phrase in row.state {
                let mapping = classify(row, phrase);
                let bits: BTreeSet<_> = mapping.bits.iter().map(|state| state.as_str().to_owned()).collect();
                required.extend(bits.iter().cloned());
                let coverage = phrases.entry(phrase).or_default();
                coverage.0 += 1;
                if mapping.complete {
                    coverage.1 += 1;
                } else {
                    unknown.push(*phrase);
                    command.unresolved.insert((*phrase).to_owned());
                }
                mappings.push(json!({"phrase":phrase,"required":bits,"complete":mapping.complete,"kind":mapping.kind,"note":mapping.note}));
            }
            command.required.extend(required.iter().cloned());
            let less: Vec<_> = required.difference(&command.declared).cloned().collect();
            paths.push(json!({
                "id":id,"family":family,"command":row.command,"path":row.path,"source":row.source,
                "pinned":command.pinned,"advertised":command.advertised,
                "required_lower_bound":required,"declared":command.declared,"less_proven":less,
                "unresolved":unknown,"mappings":mappings,
                "more":"Evaluated on the union of all paths of this command, not on an individual path."
            }));
        }
    }
    let mut compared_commands = Vec::new();
    let mut less_commands = BTreeSet::new();
    let mut more_candidates = BTreeSet::new();
    let mut unknown_commands = BTreeSet::new();
    let mut complete_commands = BTreeSet::new();
    let mut pinned_commands = BTreeSet::new();
    let mut advertised_commands = BTreeSet::new();
    let mut less_by_bit = BTreeMap::<String, BTreeSet<&str>>::new();
    let mut more_by_bit = BTreeMap::<String, BTreeSet<&str>>::new();
    for (id, command) in commands {
        let less: Vec<_> = command.required.difference(&command.declared).cloned().collect();
        let complete = command.unresolved.is_empty();
        let more: Option<Vec<_>> = complete.then(|| command.declared.difference(&command.required).cloned().collect());
        if command.advertised {
            advertised_commands.insert(id);
            if command.pinned {
                pinned_commands.insert(id);
            }
            if complete {
                complete_commands.insert(id);
            } else {
                unknown_commands.insert(id);
            }
            if !less.is_empty() {
                less_commands.insert(id);
                for bit in &less {
                    less_by_bit.entry(bit.clone()).or_default().insert(id);
                }
            }
            if let Some(more) = &more {
                if !more.is_empty() {
                    more_candidates.insert(id);
                }
                for bit in more {
                    more_by_bit.entry(bit.clone()).or_default().insert(id);
                }
            }
        }
        compared_commands.push(json!({
            "command":id,"advertised":command.advertised,"pinned":command.pinned,"paths":command.paths,
            "required_lower_bound":command.required,"declared":command.declared,"less_proven":less,
            "more_proven":Value::Null,"more_recorded_candidate":more,
            "recorded_phrases_complete":complete,"unresolved":command.unresolved,
        }));
    }
    let all: BTreeSet<_> = advertised_commands.difference(&pinned_commands).copied().collect();
    let counts = |set: &BTreeSet<&str>| {
        json!({
            "total":set.len(),"recorded_phrases_complete":set.intersection(&complete_commands).count(),
            "unresolved":set.intersection(&unknown_commands).count(),"less_proven":set.intersection(&less_commands).count(),
            "more_proven":0,"differing_proven":set.intersection(&less_commands).count(),
            "more_recorded_candidates":set.intersection(&more_candidates).count(),
        })
    };
    let phrase_report: Vec<_> = phrases.into_iter().map(|(phrase, (uses, complete))| json!({"phrase":phrase,"uses":uses,"complete_uses":complete})).collect();
    json!({
        "scope":"EditorState bits of frozen row.state phrases only. Unknowns are not matches. Object/field/eligibility/deferred-result dependencies and unrecorded context are not certified.",
        "method":"Exact whole-phrase mapping. Per-command metadata compared with the union of its path requirements. Surplus bits are candidates only when all recorded state phrases of every path are classified. They are not proven MORE: a context input can also be recorded under resolves/reads/checks or an unenumerated delegated closure.",
        "proofs":["crates/engine/engine/src/context.rs:8-42","SDK 10bd6f5 crates/kition-kit/src/lib.rs:214-236","Each path retains the frozen row.source."],
        "counts":counts(&advertised_commands),"blind_counts":counts(&all),"pinned_counts":counts(&pinned_commands),
        "class_less_editor_state_read_not_declared":less_by_bit,"class_more_editor_state_recorded_candidates":more_by_bit,
        "less_commands":less_commands,"more_candidate_commands":more_candidates,"unknown_commands":unknown_commands,
        "phrases":phrase_report,"commands":compared_commands,"paths":paths,
    })
}

// Initial unresolved pairs are named, not counted as matches. Reductions remove entries;
// a different unknown cannot replace a resolved one without failing this gate.
const UNRESOLVED_FLOOR: &str = r#"camera.createFromSolve|active_comp/selected_props/selected_layers for ct_p; CTI always selects solve frame; camera_points when points omitted
camera.deletePoints|ct_p defaults; camera_points when points omitted; removes deleted IDs from camera_points; running jobs
camera.dolly|non-active-camera branch reads/writes views3d.cams only
camera.fromView|views3d.current/cams
camera.orbit|non-active-camera branch reads/writes views3d.cams only
camera.pan|non-active-camera branch reads/writes views3d.cams only
camera.selectPoints|camera_points replace/add/toggle; revision bump
camera.setGroundPlane|ct_p defaults; camera_points when omitted; CTI always
camera.solveStatus|running/pending camera jobs; status uses CTI and selected camera points
comp.close|open_comps
comp.open|project_selection when omitted
comp.vr.createEnvironment|selection/open comp
comp.workArea|active_comp only when comp omitted; time fetched but not used unless set=begin/end
contentFill.generate|content_fill settings
contentFill.set|content_fill settings retain unspecified fields
edit.clear|text_edit and clipboards choose dispatch
edit.cut|text_edit and clipboards choose dispatch
edit.duplicate|text_edit and clipboards choose dispatch
edit.editOriginal|text_edit and clipboards choose dispatch
edit.extractWorkArea|text_edit and clipboards choose dispatch
edit.history|editor state sanitized
edit.history.goto|active_comp fallback
edit.history.goto|selected_layers/selected_props/selected_keys/selected_vertices
edit.history.goto|text_edit and clipboards choose dispatch
edit.label|text_edit and clipboards choose dispatch
edit.liftWorkArea|text_edit and clipboards choose dispatch
edit.paste|text_edit and clipboards choose dispatch
edit.pasteReversedKeyframes|text_edit and clipboards choose dispatch
edit.pasteTextFormattingOnly|text_edit
edit.pasteTextMatchFormatting|text_edit
edit.purge|active_comp fallback
edit.purge|selected_layers/selected_props/selected_keys/selected_vertices
edit.purge|text_edit and clipboards choose dispatch
edit.purgeUndo|active_comp fallback
edit.purgeUndo|selected_layers/selected_props/selected_keys/selected_vertices
edit.purgeUndo|text_edit and clipboards choose dispatch
edit.redo|active_comp fallback
edit.redo|selected_layers/selected_props/selected_keys/selected_vertices
edit.redo|text_edit and clipboards choose dispatch
edit.undo|active_comp fallback
edit.undo|selected_layers/selected_props/selected_keys/selected_vertices
edit.undo|text_edit and clipboards choose dispatch
effect.remove|filters selected_props matching removed UID
engine.batch|editor state snapshot
engine.batch|nested command reads
engine.batch|union of nested command editor-state reads
templateProps.addComment|template_props_primary/active_comp defaults; initializes primary if absent
templateProps.addGroup|template_props_primary/active_comp defaults; initializes primary if absent
templateProps.addMirror|template_props_primary/active_comp defaults
templateProps.exportTemplate|template_props_primary/active_comp defaults; render host state closure unresolved
templateProps.linkProperty|template_props_primary/active_comp defaults; selected_props if target omitted
templateProps.move|template_props_primary/active_comp defaults
templateProps.remove|template_props_primary/active_comp defaults
templateProps.rename|template_props_primary/active_comp defaults
templateProps.setName|template_props_primary/active_comp fallback; sets template_props_primary only if absent
templateProps.soloSupported|template_props_solo; writes template_props_solo and bumps revision
templateProps.unlinkProperty|template_props_primary/active_comp defaults; selected_props if target omitted
file.applyInterpretation|items_p fallback selection
file.close|active_comp/open_comps; removes open comp; may change active_comp and clear selected_layers/props/keys
file.consolidateFootage|sanitize_state filters removed editor references
file.createProxy|project_selection and active_comp only if no comp/item
file.createProxy|project_selection if no comp/item
file.importMultiple|delegated import selection/default comp state
file.interpretFootage|items_p fallback state
file.interpretFootage|project_selection; else active_comp/selected_layers if no numeric targets
file.interpretProxy|project_selection if omitted
file.newCompFromSelection|project_selection
file.reduceProject|project_selection
file.reduceProject|sanitize_state after removal
file.reloadFootage|items_p fallback selection
file.rememberInterpretation|items_p fallback project/layer selection; writes interpretation clipboard
file.removeUnusedFootage|sanitize_state filters invalid editor references
file.replaceFootage|items_p fallback selection
file.replaceWithLayeredComp|project_selection fallback/enablement
file.replaceWithPlaceholder|items_p fallback selection
file.replaceWithSolid|items_p fallback selection
file.revealInFinder|items_p fallback selection; emits OpenUrl
file.runScript|all editor state any delegated command reads/writes
file.runScript|arbitrary script-visible editor state
file.setProxy|project_selection when items/item omitted
file.setProxyNone|project_selection if omitted
file.useProxy|project_selection if omitted
footage.clearInOut|footage_panel
footage.open|footage_panel
footage.open|project_selection fallback
footage.overlayEdit|footage_panel source/range fallback
footage.overlayEdit|project_selection source fallback
footage.rippleInsertEdit|same panel/selection/part/time fallbacks as overlay
footage.setIn|footage_panel
footage.setOut|footage_panel
footage.setTime|footage_panel
jobs.list|each Apply closure editor state
jobs.wait|Apply closure editor state
layer.addItem|project_selection if item omitted
layer.mask.featherFalloff|selected_vertices
layer.mask.hideLocked|hide_locked_masks
layer.mask.motionBlur|selected_vertices
layer.newContentAwareFill|same defaults and completion selection as generate
layer.newModel|file.import editor defaults; active_comp and selected_layers when omitted
layer.newModel|project_selection if item omitted; selected_layers for omitted index; writes layer selection
layer.setText|text_edit; writes pending insertion style in addition to normal edit branch
learn.start|selected_layers only for response UI target substitution
learn.step|selected_layers only response UI target
learn.step|session.learn
learn.step|union of nested command editor-state inputs
light.controlWithCamera|same address defaults; CTI always determines released placement
mask.interpolate|mask_interp
mask.interpolationOptions|mask_interp
mask.remove|selected_vertices
mask.removeAll|selected_vertices
mediaBrowser.import|nested command time/defaults
paint.brushPreset|paint options
paint.options|paint existing options retained for omitted fields
paint.stroke|clone aligned offset initialized only if session paint.aligned and no existing offset
paint.stroke|paint all unspecified stroke options: color/tip/channels/opacity/flow/mode/dynamics/erase/duration/clone source/position/time
path.convertToBezier|selected_vertices
path.freeTransform|selected_vertices
path.rotoBezier|selected_vertices
path.setFirstVertex|selected_vertices
project.delete|filters project_selection and sanitize_state removes invalid comp/layer/prop/key selections
project.delete|project_selection if omitted
project.duplicate|project_selection if omitted; writes selection to new item IDs
project.move|project_selection when items/item omitted
project.rename|project_selection
project.select|project_selection for add; writes project_selection
project.setComment|project_selection when omitted
project.setLabel|project_selection when omitted
prop.deselect|selected_vertices
prop.select|selected_vertices
puppet.addPin|puppet density/expansion/triangles defaults unless supplied
puppet.mesh|puppet options
puppet.recordOptions|puppet options
puppet.recordPin|puppet recording defaults
puppet.recordPin|single branch state
render.preRender|active_comp/project_selection fallbacks
renderQueue.add|active_comp/project_selection fallbacks
roto.options|roto existing options retain unspecified fields
roto.stroke|rotoscope brush defaults
script.run|arbitrary script-visible state; console context if requested
scriptWindow.click|callback editor-state dependencies
scriptWindow.click|script_windows windows/dispatch
scriptWindow.close|callback state inputs
scriptWindow.close|script_windows
scriptWindow.set|callback editor-state dependencies
scriptWindow.set|script_windows
shape.newPath|shape_tool
shape.newShape|shape_tool
shape.newShape|shape_tool fill/stroke defaults
shape.toolOptions|shape_tool
text.delete|text_edit
text.edit|text_edit
text.endEdit|text_edit
text.fontFeatures|selected_layers/text_edit when font omitted
text.insert|text_edit
text.moveCaret|text_edit
text.setSelection|text_edit
track.analyze|current_track/selection defaults
track.apply|current_track/selection defaults
track.delete|current_track/selection defaults
track.editTargetDialog|part/current_track/selected_layers for eligibility
track.mask|mask_track_method default
track.options|current_track and selection defaults
track.optionsDialog|part/current_track/selected_layers for eligibility
track.reset|current_track/selection defaults
track.select|current_track
track.setPoint|current_track and selection defaults
track.setTarget|current_track and selection defaults
track.setType|current_track and selection defaults
view.lookAtAll|views3d.current/cams
view.lookAtSelected|views3d.current/cams
view.set3DViewCamera|views3d.cams existing/default"#;

const MISSING_FLOOR: &str = "";

const SURPLUS_CANDIDATE_FLOOR: &str = r#"anim.savePreset|time
camera.analyze|time
camera.move|selection
comp.new|part
comp.vr.extractCubemap|time
edit.splitLayer|part
templateProps.addMedia|active
templateProps.addProperty|active
templateProps.importTemplate|time
file.autoSave|part
file.autoSave|selection
file.autoSave|time
file.closeProject|part
file.import|selection
file.importLottie|part
file.importRecent|time
file.newProject|part
file.open|part
file.open|selection
file.open|time
file.openDemoProject|part
file.openRecent|part
file.openRecent|selection
file.openRecent|time
file.relinkFootage|part
file.relinkFootage|selection
file.relinkFootage|time
file.revert|part
file.revert|selection
file.revert|time
file.watchFolder|part
file.watchFolder|time
file.watchFolder.poll|part
file.watchFolder.poll|time
footage.check|part
footage.check|selection
footage.check|time
footage.reloadChanged|part
footage.reloadChanged|selection
footage.reloadChanged|time
keys.audioToKeyframes|selection
keys.rpfCameraImport|selection
keys.rpfCameraImport|time
layer.freezeOnLastFrame|time
layer.mask.invert|time
layer.mask.lock|time
layer.mask.mode|time
layer.mask.reset|time
layer.new3dPrimitive|time
layer.newPrimitive|time
layer.sceneEditDetection|time
layer.select|part
layer.timeReverse|time
light.spotlight|selection
prop.convertExpressionToKeyframes|time
prop.reset|time
puppet.follow|time
renderQueue.render|part
renderQueue.render|selection
renderQueue.render|time
roto.freeze|time
roto.propagate|time
roto.status|part
roto.status|selection
roto.status|time
templates.create|part
templates.create|time
track.camera|time
track.status|part
track.status|selection
track.status|time
track.warpStabilizer|time
warp.analyze|time
warp.status|part
warp.status|selection
warp.status|time"#;

#[test]
fn frozen_editor_state_comparison_reports_unknowns_separately() {
    let document = MotionDocument::create("r2-state", 8, 8).unwrap();
    let answer = audit(&document);
    println!("R2 EDITOR STATE {}", answer["counts"]);
    if let Ok(path) = std::env::var("MOTION_R2_STATE_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&answer).unwrap()).unwrap();
    }
    assert_eq!(answer["paths"].as_array().unwrap().len(), 652);
    assert_eq!(answer["counts"]["total"], 486);
    assert_eq!(answer["pinned_counts"]["total"], 3);
    let allowed_unknown: BTreeSet<_> = UNRESOLVED_FLOOR.lines().map(str::to_owned).collect();
    let allowed_missing: BTreeSet<_> = MISSING_FLOOR.lines().map(str::to_owned).collect();
    let allowed_candidates: BTreeSet<_> = SURPLUS_CANDIDATE_FLOOR.lines().map(str::to_owned).collect();
    let mut unknown = BTreeSet::new();
    let mut missing = BTreeSet::new();
    let mut candidates = BTreeSet::new();
    for command in answer["commands"].as_array().unwrap() {
        let id = command["command"].as_str().unwrap();
        unknown.extend(command["unresolved"].as_array().unwrap().iter().map(|phrase| format!("{id}|{}", phrase.as_str().unwrap())));
        missing.extend(command["less_proven"].as_array().unwrap().iter().map(|state| format!("{id}|{}", state.as_str().unwrap())));
        if let Some(states) = command["more_recorded_candidate"].as_array() {
            candidates.extend(states.iter().map(|state| format!("{id}|{}", state.as_str().unwrap())));
        }
        if !command["unresolved"].as_array().unwrap().is_empty() {
            assert!(command["more_recorded_candidate"].is_null(), "unknown state was treated as an exact match: {command}");
        }
        assert!(command["more_proven"].is_null(), "recorded state phrases alone cannot prove held MORE: {command}");
    }
    assert!(unknown.is_subset(&allowed_unknown), "unresolved state comparison grew: {:?}", unknown.difference(&allowed_unknown).collect::<Vec<_>>());
    assert!(missing.is_subset(&allowed_missing), "missing state declaration grew: {:?}", missing.difference(&allowed_missing).collect::<Vec<_>>());
    assert!(
        candidates.is_subset(&allowed_candidates),
        "surplus state candidate set grew: {:?}",
        candidates.difference(&allowed_candidates).collect::<Vec<_>>()
    );
    assert_eq!(answer["counts"]["less_proven"].as_u64().unwrap(), 0);
    assert!(answer["counts"]["unresolved"].as_u64().unwrap() <= 139);
    assert!(answer["counts"]["more_recorded_candidates"].as_u64().unwrap() <= 49);
}
