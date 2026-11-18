use std::collections::BTreeSet;

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;

pub(crate) const ROWS: &[(&str, &str)] = &[
    (
        "prop.set",
        "path=ordinary-static; resolves=comp id/numeric-string/name or active,layer id/numeric-string/#index/name or first selected,prop uid or path/prop-string with bare Transform fallback; reads=editor time_of(comp),prefs default_spatial_linear,layer start_time/stretch,property keys/value/spatial/hold_only/ui; holds=resolved property identity,static value,type,slider bounds,keys empty; checks=has_layers,value coercion,key time range/timeBase/stretch; writes=property.value clamped to slider range; source=commands/prop.rs:set,with_prop,prop_ref,key_time;project/src/props.rs:set_value_at",
    ),
    (
        "prop.set",
        "path=ordinary-animated; resolves=prop_ref,time supplied in layer(default)/comp base snapped on comp frame grid else editor time_of(comp) mapped to layer; reads=editor time_of(comp) even with explicit time for coercion,current value_at CTI,layer start_time/stretch,comp frame_rate,property keys/value/spatial/hold_only/ui,prefs default_spatial_linear; holds=property type/ui,animation state,curve needed by CTI coercion,same-time destination key value and interpolation,old key-time set; checks=has_layers,valid conversion/coercion; writes=add/replace key value preserving existing interpolation/label/roving,linear new spatial tangents when preference on; source=commands/prop.rs:set,snapped_key_time,with_prop;project/src/props.rs:set_value_at;keyframe/src/lib.rs:set_key",
    ),
    (
        "prop.set",
        "path=separated-position; resolves=ordinary prop_ref then Position uid plus positionX existence,provided array dispatches X/Y and Z only for 3D with third input; reads=transform children,switches.three_d/source,each axis ordinary-static/animated dependencies; holds=separation membership,3D eligibility,each assigned axis value or destination key and type; checks=each recursive set; writes=axis values/keys,returned array per written dimension; source=commands/prop.rs:set;project/src/lib.rs:is_3d",
    ),
    (
        "prop.set",
        "path=selected-twins; resolves=selected true plus source layer selected finds same match path and value kind/component count on other selected unlocked layers; reads=editor selected_layers,source match path/value shape,twin locked flags/property types,all ordinary/separated dependencies,source and twin evaluated CTI components when offset true; holds=target set,property path/type/lock eligibility,assigned twin values/keys,offset curves; checks=source set errors propagate,twin set errors ignored; writes=source and eligible twin properties grouped unless merge,offset applied relative to each twin CTI value; source=commands/prop.rs:selected_twins,set_selected,current_components,twin_params;commands/app_more.rs:grouped",
    ),
    (
        "prop.toggleAnimation",
        "path=single-or-selected-twins; resolves=prop_ref,selected true repeats same match path on selected unlocked compatible twins; reads=editor time_of(comp)/selected_layers,prefs default_spatial_linear,layer start_time/stretch,property static_only/hold_only/spatial,value/keys,explicit value else keys empty; holds=property eligibility,animation state,static value when enabling,evaluated CTI curve when disabling,all assigned value/keys and twin target eligibility; checks=has_layers,!static_only; writes=one CTI key when enabling empty property,or value_at CTI and clear keys when disabling,twin value forced to source result; source=commands/prop.rs:toggle_anim_selected,toggle_anim,with_prop;project/src/props.rs:set_animated",
    ),
    (
        "prop.addKey",
        "path=explicit-time-with-value; resolves=prop_ref,time in layer(default)/comp base maps to comp snaps then layer; reads=editor time_of(comp) via with_prop,prefs default_spatial_linear,comp frame_rate,layer start_time/stretch,in_point/out_point only explicit-base reply,property curve at written time for type/shape,hold_only/spatial/existing key times; holds=property type/hold policy,same-time key value and interpolation,old key-time set,placement conversion; checks=has_layers,finite in-range time/nonzero stretch,value coercion; writes=insert/replace key preserving existing metadata,new spatial linear default,count reply; source=commands/prop.rs:add_key,key_time,key_timing,with_prop;keyframe/src/lib.rs:set_key",
    ),
    (
        "prop.addKey",
        "path=value-omitted-or-time-omitted; resolves=prop_ref,supplied snapped time else editor time_of(comp) through layer time; reads=above plus property.value_at written time and full interpolation neighbors; holds=sampled curve,same-time assigned key value/metadata,layer timing and explicit-base active-range reply; checks=has_layers,valid supplied time; writes=sampled value as key,hold_only forces Hold; source=commands/prop.rs:add_key,with_prop;project/src/props.rs:value_at",
    ),
    (
        "prop.toggleKey",
        "path=existing-key; resolves=prop_ref,CTI time_of(comp) mapped by start_time/stretch,optional selected compatible unlocked twins; reads=editor selected_layers/time,prefs spatial default,keys exact CTI membership,sole key value; holds=destination key existence/value/metadata,key count,static value if sole-key deletion,twins eligibility; checks=has_layers; writes=remove CTI key,copy sole key value to static value,twins remove/add to match source added boolean; source=commands/prop.rs:toggle_key_selected,toggle_key_at,with_prop",
    ),
    (
        "prop.toggleKey",
        "path=missing-key; resolves=prop_ref and optional selected twins at CTI; reads=property evaluated CTI curve,hold_only/spatial,prefs spatial default,layer timing; holds=CTI absence,sampled curve,assigned key and twin eligibility; checks=has_layers; writes=insert sampled key,hold-only Hold,new spatial linear default; source=commands/prop.rs:toggle_key_selected,toggle_key_at,with_prop",
    ),
    (
        "keys.toggleTransform",
        "path=transform-variants; resolves=layers explicit or selected,prop anchor/position/scale/rotation/opacity,Position expands separated axes,3D Rotation expands orientation/X/Y/Z; reads=editor time_of(comp),layer timing,source/three_d switch,transform member existence,exact CTI keys/count/curve,hold_only; holds=target members,3D/separation eligibility,each assigned key or sole static value; checks=has_layers,valid prop enum; writes=toggle each target key,sole-key deletion retains static value,reply per target key boolean; source=commands/prop.rs:toggle_transform_key,toggle_key_at;commands/mod.rs:layers_p",
    ),
    (
        "prop.setExpression",
        "path=explicit; resolves=prop_ref; reads=editor layer selection only fallback,prefs-independent compact reference built from property node match/name chain,old expr when toggle or enabled-only,expr_check hook; holds=property identity,assigned expr field including explicit replacement,node-chain names used for default expression; checks=has_layers,syntax error disables supplied text; writes=expr None for empty text,replace text/enabled,enable existing/default own reference,or toggle presence,reply expression text and optional syntax error/toast; source=commands/prop.rs:set_expr,apply_expr;commands/link.rs:reference",
    ),
    (
        "prop.setExpression",
        "path=selected-properties; resolves=comp explicit or active,editor selected_props leaves or expanded groups in active comp; reads=selected property/group membership and reference node chains,old expr per property,syntax hook; holds=target set,assigned expressions,default reference names; checks=has_layers,nonempty target set; writes=each expression in one edit,array reply; source=commands/prop.rs:set_expr;commands/mod.rs:selected_leaf_props",
    ),
    (
        "prop.reset",
        "path=default-parameter; resolves=prop_ref; reads=property value type,computed own default even if parameter supplied,ancestor effect kind/param match id or Transform/source kind/source dimensions/comp dimensions; holds=assigned static value/keys/expr,type,default inputs actually selected; checks=has_layers; writes=clear keys and expr,set coerced supplied default else own known default else retain static value; source=commands/prop.rs:reset,default_value;render/src/eval.rs:source_size,effect_bounds",
    ),
    (
        "prop.reset",
        "path=own-or-unknown-default; resolves=prop_ref,ancestor Effect definition parameter or noncamera/nonlight Transform defaults; reads=source item dimensions or Null size,comp dimensions,effect bounds and definition,property match id; holds=property value/keys/expr and default source fields; checks=has_layers; writes=clear keys/expr,value to own default if known,unknown default preserves value; source=commands/prop.rs:default_value,reset;commands/layer.rs:transform_default",
    ),
    (
        "prop.convertExpressionToKeyframes",
        "path=sample-range; resolves=prop_ref; reads=comp frame_rate,layer in_point/out_point/start_time/stretch,property value/keys/expr/spatial,expression host and its arbitrary project/footage dependencies,separated Position axes; holds=entire evaluated dependency closure,assigned keys and expression enabled state; checks=has_layers; writes=one evaluated key per comp frame in active range sorted by layer time,disable existing expression,reply key count; source=commands/prop.rs:convert_expr_to_keys;render/src/eval.rs:EvalCtx::value",
    ),
    (
        "prop.select",
        "path=editor-selection; resolves=layer_p,prop numeric uid accepted directly else prop_ref; reads=editor selected_layers/props/vertices/keys,time,mask/path group membership and curve at layer time; holds=none(document write); checks=has_layers; writes=editor selected_props/layers,clear vertices when replacing,select all mask/path vertices,optional selected_keys of property; source=commands/prop.rs:select_prop,path_points",
    ),
    (
        "prop.deselect",
        "path=editor-selection; resolves=layer_p,prop numeric uid direct else prop_ref; reads=editor selection/time,path owner and sampled vertices; holds=none(document write); checks=has_layers; writes=remove selected property/keys and matching path vertices; source=commands/prop.rs:deselect_prop,path_points",
    ),
    (
        "keys.select",
        "path=editor-selection; resolves=active comp,each nested layer ref id/index/name,prop uid/path or bare Transform fallback,timeBase layer(default)/comp unsnapped,nearest stored key; reads=editor selection when add/toggle,layer timing/locked,property key times,explicit-base reply active range; holds=none(document write); checks=has_comp,known params,all layer/property/time references valid,nonempty keys per target; writes=editor selected_keys excluding locked layers,optional associated selected_props/layers,reply selected count/timing warnings; source=commands/prop.rs:select_keys,key_time",
    ),
    (
        "prop.renameGroup",
        "path=instance; resolves=layer_p plus prop group uid; reads=group existence/kind; holds=group identity/kind,assigned name; checks=has_layers,Indexed/Effect/Mask/Tracker instance,nonblank supplied name; writes=group.name; source=commands/prop_groups.rs:group_ref,rename",
    ),
    (
        "prop.setGroupEnabled",
        "path=instance-explicit-or-toggle; resolves=layer_p plus prop group uid; reads=group kind,old enabled for omitted value; holds=group identity/kind,assigned enabled even with explicit value; checks=has_layers,instance group; writes=group.enabled explicit value else negation; source=commands/prop_groups.rs:group_ref,set_enabled",
    ),
    (
        "prop.removeGroup",
        "path=instance; resolves=layer_p plus instance group uid; reads=group kind,parent membership,editor selected_props; holds=removed group subtree and parent membership; checks=has_layers,instance; writes=remove group subtree,retain editor selected_props except removed group uid; source=commands/prop_groups.rs:remove,remove_groups",
    ),
    (
        "prop.duplicateGroup",
        "path=instance; resolves=layer_p plus instance group uid; reads=group full subtree,parent order/project next_id; holds=source subtree,parent insertion place; checks=has_layers,instance and parent membership; writes=insert cloned fresh-uid group after original,next_id,reply new prop uid; source=commands/prop_groups.rs:duplicate;project/src/props.rs:reassign_uids",
    ),
    (
        "prop.moveGroup",
        "path=instance; resolves=layer_p plus instance group uid,index 1-based among siblings; reads=group kind,parent children order/count; holds=source/destination sibling placement and group membership; checks=has_layers,instance,index within 1..children.len; writes=reorder parent children without changing group content; source=commands/prop_groups.rs:move_to",
    ),
    (
        "keys.move",
        "path=single-or-merged-gesture; resolves=active comp,editor selected_keys or saved key_move.from with matching history merge; reads=editor selected_keys,key_move/history base,total,comp frame_rate,layer start_time/stretch/locked,property entire key list/spatial and roving-neighbor geometry; holds=selected keys and colliding destination keys,time placement/locked flags,roving run values/tangents/times; checks=has_keys,delta supplied; writes=move selected keys in comp time snapped to grid,replace collision keys,retime all roving runs,remap selected_keys,merged gesture may restore project history base; source=commands/prop.rs:move_keys,shift_keys;keyframe/src/lib.rs:retime_roving",
    ),
    (
        "keys.delete",
        "path=selected; resolves=active comp/editor selected_keys exact stored key times; reads=property key membership/count,sole-key value; holds=deleted key content/metadata,sole-key static value; checks=has_keys; writes=delete matched keys,copy last key to static value,clear editor selected_keys,reply original selection length; source=commands/prop.rs:delete_keys",
    ),
    (
        "keys.easyEase",
        "path=selected; resolves=active comp/editor selected_keys grouped by layer/prop exact times,which defaults both; reads=selected key value dims,property spatial,whole roving runs after edit; holds=assigned side interp/ease/auto_bezier and roving time/interp changes,roving geometry; checks=has_keys; writes=in/out Bezier EASY by which,auto_bezier false,retime_roving and remap selection; source=commands/prop.rs:ease,edit_keys;keyframe/src/lib.rs:easy_ease,retime_roving",
    ),
    (
        "keys.easyEaseIn",
        "path=selected; resolves=active comp/editor selected_keys,which forced in; reads=key value dims/property spatial,roving runs; holds=assigned in_interp/in_ease/auto_bezier,roving geometry/time/interp; checks=has_keys; writes=in Bezier EASY,auto false,roving retime/remap; source=commands/prop.rs:specs,ease,edit_keys;keyframe/src/lib.rs:easy_ease",
    ),
    (
        "keys.easyEaseOut",
        "path=selected; resolves=active comp/editor selected_keys,which forced out; reads=key value dims/property spatial,roving runs; holds=assigned out_interp/out_ease/auto_bezier,roving geometry/time/interp; checks=has_keys; writes=out Bezier EASY,auto false,roving retime/remap; source=commands/prop.rs:specs,ease,edit_keys;keyframe/src/lib.rs:easy_ease",
    ),
    (
        "keys.toggleHold",
        "path=selected; resolves=active comp/editor selected_keys exact times; reads=selected key auto/continuous/in_interp/out_interp/out_ease,property spatial and roving list; holds=out_interp and deciding flags/eases,roving geometry/time/interp; checks=has_keys; writes=out Hold toggle to Bezier when easing present else Linear,retime_roving/remap; source=commands/prop.rs:toggle_hold,edit_keys",
    ),
    (
        "keys.interpolation",
        "path=temporal-and-spatial; resolves=active comp/editor selected_keys exact times; reads=key dims,current in/out ease/continuous/auto flags,spatial_handles including neighbor values/tangents,all roving runs; holds=each assigned temporal/spatial interpolation/ease/flag field,computed neighboring handles/continuous speeds,roving time/geometry; checks=has_keys,unrecognized option values silently do nothing; writes=requested temporal/spatial/roving flags,allocate ease defaults as needed,continuous average speeds,auto disables continuous,retime_roving/remap; source=commands/prop.rs:interpolation,edit_keys;keyframe/src/lib.rs:spatial_handles,retime_roving",
    ),
    (
        "keys.velocity",
        "path=selected; resolves=active comp/editor selected_keys exact times,numeric or per-dimension params,continuous copies omitted side inputs; reads=key dims,side_ease current presentation with neighbor slope/values/times,property spatial for roving; holds=assigned ease/interp/continuous/auto flags,neighbor presentation and roving runs; checks=has_keys,influence clamped .001..1; writes=specified speeds/influences,Bezier affected sides,continuous optional,auto false,retime_roving/remap; source=commands/prop.rs:velocity,edit_keys;keyframe/src/lib.rs:side_ease",
    ),
    (
        "keys.copy",
        "path=editor-clipboard; resolves=active comp/editor selected_keys groups,earliest selected time in comp space; reads=layer timing,selected full key records and match paths; holds=none(document write); checks=has_keys,at least one selected layer exists; writes=editor key_clipboard with comp-relative key times,clip_is_keys true; source=commands/anim.rs:copy",
    ),
    (
        "keys.paste",
        "path=single-property-clip; resolves=comp,layers explicit or selected else clipboard source layers,time explicit/frame else CTI,explicit prop uid or first-target path else selected compatible leaf else clipboard match path; reads=editor key_clipboard/selected_props/layers/time,property value kind/component count/static_only/hold_only/spatial,key list,layer timing; holds=target resolution/type/static policy,assigned destination keys and preserved neighboring keys/roving inputs; checks=has_key_clip,nonempty compatible plan; writes=insert/replace complete copied keys after target value coercion/hold conversion,retime_roving,editor selected_keys; source=commands/anim.rs:paste,compatible;commands/mod.rs:time_p,layers_p",
    ),
    (
        "keys.paste",
        "path=multiple-property-or-layer-clips; resolves=layers as above,match clipboard paths,paired source-to-target when multiple source count equals target count; reads=clipboard source order/keys,property kind/dims/static/hold/spatial,layer timing,CTI when time absent; holds=paired target set,match path identity/type,assigned destination keys,roving inputs; checks=compatible plan; writes=complete copied keys replacing collisions,roving retime,selection and count; source=commands/anim.rs:paste",
    ),
    (
        "keys.selectAll",
        "path=editor-visible-or-selected-properties-or-layers; resolves=layers_p,visible param takes priority,else omitted layer refs with selected_props,else selected layers or all comp layers; reads=property key times,layer locked,editor selected_props/layers; holds=none(document write); checks=has_comp; writes=editor selected_keys excluding locked layers; source=commands/anim.rs:select_all",
    ),
    (
        "keys.nudge",
        "path=selected; resolves=active comp/editor selected_keys or matching merged gesture,frames param; reads=comp frame_duration plus all shift_keys timing/lock/key/roving/history inputs; holds=assigned moved/collision keys and roving dependencies; checks=has_keys,integer frames; writes=shift_keys by saturated frame_duration*frames; source=commands/anim.rs:nudge;commands/prop.rs:shift_keys",
    ),
    (
        "keys.nudgeForward",
        "path=selected; resolves=active comp/editor selected_keys; reads=comp frame_duration,layer timing/locked,selected/collision keys,roving geometry; holds=moved/collision keys and roving dependencies; checks=has_keys; writes=shift_keys by one comp frame; source=commands/anim.rs:specs,nudge;commands/prop.rs:shift_keys",
    ),
    (
        "keys.nudgeBackward",
        "path=selected; resolves=active comp/editor selected_keys; reads=comp frame_duration,layer timing/locked,selected/collision keys,roving geometry; holds=moved/collision keys and roving dependencies; checks=has_keys; writes=shift_keys by minus one comp frame; source=commands/anim.rs:specs,nudge;commands/prop.rs:shift_keys",
    ),
    (
        "keys.nudgeForward10",
        "path=selected; resolves=active comp/editor selected_keys; reads=comp frame_duration,layer timing/locked,selected/collision keys,roving geometry; holds=moved/collision keys and roving dependencies; checks=has_keys; writes=shift_keys by ten comp frames; source=commands/anim.rs:specs,nudge;commands/prop.rs:shift_keys",
    ),
    (
        "keys.nudgeBackward10",
        "path=selected; resolves=active comp/editor selected_keys; reads=comp frame_duration,layer timing/locked,selected/collision keys,roving geometry; holds=moved/collision keys and roving dependencies; checks=has_keys; writes=shift_keys by minus ten comp frames; source=commands/anim.rs:specs,nudge;commands/prop.rs:shift_keys",
    ),
    (
        "keys.set",
        "path=value-and-or-time; resolves=prop_ref,time layer(default)/comp unsnapped nearest key,newTime same base snapped then clamped strictly between neighbors; reads=editor selected_keys,comp frame_rate,layer timing/in/out for reply,key value coercion/type,neighbor times,spatial/roving entire list; holds=nearest-key identity/time/value,assigned value/time,neighbor clamp bounds,roving geometry/time; checks=has_comp,finite valid time/conversion,property has keys,value coercion; writes=key value/time,retime_roving,selection remap,reply comp/layer timing plus active-range warning; source=commands/anim.rs:set_key,nearest_key;commands/prop.rs:key_time,key_timing",
    ),
    (
        "keys.setEase",
        "path=one-side-or-continuous; resolves=prop_ref,time raw layer seconds nearest key,side required in/out,dim optional; reads=key spatial/value dims,side_ease and adjacent keys/geometry,current ease arrays,continuous flag; holds=nearest key time and assigned interp/ease/auto flags,opposite side when continuous,current effective speeds/influences for unspecified dims; checks=has_comp,key exists,valid side; writes=selected side ease/speed/influence,Bezier and auto false,continuous mirrors speeds to opposite side and both Bezier; source=commands/anim.rs:set_ease;keyframe/src/lib.rs:side_ease",
    ),
    (
        "keys.selectEqual",
        "path=editor-selection; resolves=active comp/editor selected_keys groups; reads=selected key values and every key value/time in same properties; holds=none(document write); checks=has_keys; writes=extend selected_keys with equal-value keys; source=commands/anim.rs:select_related",
    ),
    (
        "keys.selectPrevious",
        "path=editor-selection; resolves=active comp/editor selected_keys groups; reads=selected key earliest times,property key times; holds=none(document write); checks=has_keys; writes=extend selected_keys before earliest selected key per property; source=commands/anim.rs:select_related",
    ),
    (
        "keys.selectFollowing",
        "path=editor-selection; resolves=active comp/editor selected_keys groups; reads=selected key latest times,property key times; holds=none(document write); checks=has_keys; writes=extend selected_keys after latest selected key per property; source=commands/anim.rs:select_related",
    ),
    (
        "keys.timeReverse",
        "path=selected; resolves=active comp/editor selected_keys exact times per property; reads=selected complete keys and first/last range,other key times for collisions; holds=selected full key records and collision destinations; checks=has_keys; writes=reverse selected times,swap in/out interp/ease/spatial tangent sides,replace collisions,editor selected_keys; source=commands/anim.rs:time_reverse;keyframe/src/lib.rs:time_reverse",
    ),
    (
        "keys.wiggle",
        "path=selected-run; resolves=active comp/editor selected_keys grouped sorted,first/last exact stored indices; reads=comp frame_rate,layer start/stretch,property value interpolates/spatial/full key curve including outside neighbors,uid seeded stream; holds=range key contents and all sampled interpolation dependencies,assigned replacement key list; checks=has_keys,at least two selected times per property,valid apply/noise/dimensions,frequency>0; writes=replace interior with seeded spatial-value or temporal jitter keys,keep outside/end keys,editor selected_keys in range,reply before/after; source=commands/anim_tools.rs:run_on_selection,wiggle_keys,parse_wiggle",
    ),
    (
        "keys.smooth",
        "path=selected-run; resolves=active comp/editor selected_keys,first/last exact indices; reads=comp frame_rate,layer timing,property interpolates/spatial,whole original evaluated curve sampled each comp frame; holds=sampled curve and boundary/outside keys,assigned keys; checks=has_keys,at least three selected times; writes=replace interior with minimal auto-Bezier or spatial linear sample keys within tolerance,editor selected_keys in range; source=commands/anim_tools.rs:run_on_selection,smooth_keys,sample_curve",
    ),
    (
        "motion.sketch",
        "path=recorded-path; resolves=layer_p,start explicit comp seconds else editor time; reads=comp frame_rate/duration,layer timing,Position value dimensionality/Z and existing keys; holds=Position value/type/Z,keys replaced within capture span,kept keys used by smoothing; checks=has_comp,Position exists,at least two valid points; writes=Position keys one per frame clamped to comp end with optional smoothing,preserve outside keys,editor selected_keys,reply count; source=commands/anim_tools.rs:sketch,smooth_keys",
    ),
    (
        "anim.savePreset",
        "path=external-file; resolves=active comp/editor selected_props expanded leaves and selected effects; reads=selected property complete value/keys/expr,match paths,effect full groups,earliest selected property key time,editor time/selection; holds=selected source content/path/order for exported result; checks=has_selected_props,nonempty selected props/effects,path supplied; writes=external preset JSON file,no document mutation; source=commands/animation.rs:save_preset,selected_effects;commands/mod.rs:selected_leaf_props",
    ),
    (
        "anim.applyPreset",
        "path=effect-append; resolves=layers_p,inline preset else external path,preset tag required; reads=editor time,layer start/stretch,effects group existence,project next_id,recent prefs path; holds=target effects membership for append placement,layer timing; checks=has_layers,nonempty targets,valid preset tag; writes=append fresh-uid effect groups,next_id,recent preset preference,reply applied/skipped; source=commands/animation.rs:apply_preset;project/src/props.rs:reassign_uids",
    ),
    (
        "anim.applyPreset",
        "path=property-overwrite; resolves=each preset property match path on each resolved layer; reads=editor time,layer timing,target value discriminant,source preset keys/value/expr; holds=path/type eligibility,assigned target value/keys and expr when preset has expression; checks=matching value discriminants else skip; writes=replace value and all keys offset by layer CTI,replace expr only when preset expr present,reply applied/skipped; source=commands/animation.rs:apply_preset",
    ),
    (
        "anim.addKeyframe",
        "path=selected-leaves; resolves=active comp/editor selected_props expanded groups; reads=editor time,layer timing,static_only,keys empty/value/curve/hold_only; holds=target set and eligibility,assigned key/value and animation state; checks=has_selected_props,nonempty selected_leaf_props,skip static_only; writes=enable animation or sample/add key at CTI,reply count; source=commands/animation.rs:add_keyframe;project/src/props.rs:set_animated,set_value_at",
    ),
    (
        "keys.exponentialScale",
        "path=selected-scale-range; resolves=active comp/editor selected_keys only Transform scale uid with >=2 times; reads=comp frame_duration,property sampled endpoint values and complete retained keys; holds=scale identity,range endpoint curve and all replaced keys; checks=has_keys,at least one eligible Scale pair; writes=frame-spaced exponential Scale Vec3 keys with sign/zero linear fallback,retain outside,extend/dedup selection; source=commands/animation.rs:exponential_scale",
    ),
    (
        "keys.audioToKeyframes",
        "path=sample-comp-audio; resolves=comp_id; reads=comp work_area/frame_rate/dimensions/renderer/layer stack,editor selected_layers for null insertion,prefs labels,all audible nested layers source/timing/switches/solo/audio levels/effects/expressions/footage; holds=complete audio-evaluation closure,work_area and insertion place; checks=has_comp,slider effect exists; writes=new Audio Amplitude null above selection,three fresh slider effects with every-frame keys,next_id,editor layer/prop/key selections,reply peak/frames; source=commands/keys_more.rs:audio_to_keys;render/src/audio.rs:mix_comp;commands/layer.rs:new_null,insert_layer_at",
    ),
    (
        "keys.rpfCameraImport",
        "path=external-camera; resolves=comp_id,path JSON/CSV samples frameRate/time/frame defaults; reads=comp frame_rate/width/height/renderer/duration/layer stack,editor selected_layers insertion place,prefs default label,external camera bytes; holds=comp geometry/timing and insertion place used to create camera; checks=has_comp,no unsupported binary suffix,nonempty valid samples; writes=new one-node camera above selection,position/orientation/optional zoom keys relative to new start,next_id,editor selection,reply sample count; source=commands/keys_more.rs:rpf_camera_import,parse_camera;commands/three_d.rs:new_camera;commands/layer.rs:insert_layer_at",
    ),
    (
        "keys.transform",
        "path=selected-or-fromStart-gesture; resolves=active comp/editor selected_keys or matching history/key_move gesture base,default anchor earliest selected comp time; reads=comp frame_rate,layer start/stretch,selected key complete values/time,other key times for collisions,property spatial/roving,history base/fromStart; holds=selected keys,collision destinations,roving geometry,placement timing; checks=has_keys,positive finite timeScale; writes=map times snapped nonnegative comp frames,optional dimension value transform,drop colliding unselected keys/dedup,retime_roving,editor selection,gesture may restore saved project; source=commands/key_transform.rs:transform",
    ),
    (
        "keys.setSpatialTangents",
        "path=nearest-spatial-key; resolves=prop_ref,time raw layer seconds nearest key; reads=property spatial,key times/current tangents,auto tangents from neighbor values,roving entire list; holds=nearest key and assigned spatial_auto/continuous/in/out,computed opposite tangent and roving inputs; checks=has_comp,spatial property,one valid tangent side,key exists; writes=spatial_auto false,one side mirrors unless break else preserve current opposite,roving retime; source=commands/key_transform.rs:spatial_tangents;keyframe/src/lib.rs:spatial_tangents,retime_roving",
    ),
    (
        "keys.setLabel",
        "path=selected; resolves=active comp/editor selected_keys exact times,label built-in name or numeric 0..16; reads=key membership,property spatial/full roving runs; holds=assigned key label and roving geometry/time/interp; checks=has_keys,valid label; writes=key.label,edit_keys reinsertion/retime_roving and selection,reply label/count; source=commands/key_labels.rs:set_label;commands/prop.rs:edit_keys",
    ),
    (
        "keys.selectLabelGroup",
        "path=editor-scope-selection; resolves=active comp,labels of selected keys,scope selected/all/visibleSelected/visibleAll; reads=editor selected_keys/layers,property labels/times,optional visible uid list; holds=none(document write); checks=has_comp,one existing selected key,valid scope; writes=editor selected_keys sharing selected labels within scope,locked not filtered; source=commands/key_labels.rs:select_label_group",
    ),
    (
        "keys.applyEase",
        "path=curve; resolves=prop_ref,from/to layer(default)/comp base snapped; reads=comp frame_rate,layer timing,property hold_only/value interpolates/spatial,key count/times/values/interp/tangents and neighboring auto tangents for segment length; holds=eligibility,all eased side interp/ease/auto/continuous and inserted/colliding key times,segment geometry; checks=has_comp,valid curve,at least two keys,consecutive pair in range; writes=ease each eligible segment,insert spatial overshoot turning keys,clear editor selected_keys,reply segments/inserted/total keys; source=commands/promo_ease.rs:apply_ease,ease_segment;keyframe/src/lib.rs:spatial_segment_length",
    ),
    (
        "keys.applyEase",
        "path=spring; resolves=prop_ref,preset named spring or numeric damping/stiffness/mass defaults,from/to still validated; reads=comp frame_rate,layer timing,property hold_only/value interpolates/key count; holds=property eligibility/key count,assigned expr field; checks=has_comp,at least two interpolating nonhold keys; writes=replace expr with enabled spring expression,reply spring constants; source=commands/promo_ease.rs:apply_ease,shape_p,spring_expr",
    ),
    (
        "prop.separateDimensions",
        "path=split; resolves=layer_p,value true or toggle when no positionX; reads=Transform membership/order,Position full value/keys/spatial and derived per-axis velocities,project next_id; holds=Position data/separation membership/insertion place; checks=has_layers,Transform/Position exists; writes=insert X/Y/Z scalar properties with fresh uids after Position,split full keys/eases,three_d_only Z,next_id; source=commands/link.rs:separate,split_keys",
    ),
    (
        "prop.separateDimensions",
        "path=join; resolves=layer_p,value false or toggle when positionX exists; reads=all separated axis values/keys/evaluated curves,union key times; holds=axis content and removed members,assigned combined Position value/keys; checks=has_layers,Transform/Position exists; writes=combined Position Vec3 value/keys,remove axis properties,next_id retained; source=commands/link.rs:separate",
    ),
    (
        "prop.separateDimensions",
        "path=already-requested-state; resolves=layer_p,value equal existing separation state; reads=Transform/positionX presence; holds=separation membership; checks=has_layers,Transform exists; writes=none(document),reply target bool; source=commands/link.rs:separate",
    ),
    (
        "prop.pickWhip",
        "path=replace-expression; resolves=source prop_ref,target nested prop_ref then target uid in source comp,target dimension optional; reads=target layer name and complete property node name/match chain,prefs expression_pick_whip_compact,source/target value type/shown_dims/3D source or switch; holds=reference-name chain and dimension eligibility,source assigned expr; checks=has_layers,no self-reference,valid target dimension; writes=source expression via prop.setExpression with scalar/vector adaptation and syntax hook; source=commands/link.rs:pick_whip,reference,dims;commands/prop.rs:set_expr",
    ),
    (
        "prop.pickWhip",
        "path=insert-in-supplied-expression; resolves=source/target prop_ref,range char indices clamped to supplied expression length else end; reads=reference-name chain,prefs compact,source/target dimensionality as above; holds=target name/path/dimensionality,source assigned expr; checks=has_layers,no self,valid dimension and two-element range; writes=insert target reference into provided text then prop.setExpression; source=commands/link.rs:pick_whip,reference",
    ),
    // Pinned from the declaration after review: later families followed guard-summary exposure.
    // Only the preceding 66 rows (52 registry writes) were committed fully blind in 0087bd3b.
    (
        "layer.addTextAnimator",
        "path=new-animator; resolves=layer_p,properties/property array or string defaults opacity,name supplied else Animator children.len+1; reads=editor selected text layer eligibility,text/animators membership/count,text/perChar3d.value,project next_id; holds=source Text/animators eligibility,perChar3d value,name-generating count,animators append placement; checks=has_text_layer,recognized property set unless explicitly empty; writes=append fresh animator with default Range Selector and requested companion properties,next_id,reply animator uid; source=commands/text_anim.rs:add_animator,add_props;project/src/build.rs:text_animator,text_anim_props",
    ),
    (
        "layer.addTextAnimator",
        "path=perChar3d-property; resolves=layer_p,any requested perChar3d redirects enable_per_char with layer/comp only; reads=text source,perChar3d property/animator structure and dimensional values; holds=assigned perChar3d,three_d switch,affected animator fields/membership; checks=has_text_layer,Text source; writes=enable per-character 3D and layer three_d,expand existing animator components and rotation properties,next_id; source=commands/text_anim.rs:add_animator,enable_per_char,set_animators_3d",
    ),
    (
        "layer.addTextAnimatorProperty",
        "path=resolved-animator; resolves=layer_p,animator uid preferred then 1-based group index,else selected descendant property owner else last animator; reads=editor selected_props,animator list/order,perChar3d.value,existing property match ids; holds=animator resolution/membership,perChar3d,absence/presence requested companion properties; checks=has_animator,all property kinds known; writes=append missing requested/companion properties,create properties group if absent,next_id increments even for allocated skipped duplicates; source=commands/text_anim.rs:add_animator_prop,animator_uid,add_props",
    ),
    (
        "layer.addTextSelector",
        "path=resolved-animator; resolves=layer_p,animator uid/index or selected-descendant owner or last,kind default range; reads=editor selected_props,animator list/order,selectors membership and same-kind count; holds=animator resolution,selector insertion/name-count inputs; checks=has_animator,kind range/wiggly/expression; writes=create selectors group at first child if missing,append fresh selector named by kind count,next_id,reply selector/animator uid; source=commands/text_anim.rs:add_selector,animator_uid;project/src/build.rs:text_selector",
    ),
    (
        "text.addSelector",
        "path=resolved-animator; resolves=layer_p,animator uid/index or selected-descendant owner or last,kind default range; reads=editor selected_layers/props,animators list/order/selectors kind count; holds=animator identity/eligibility and selector append/name-count inputs; checks=has_text_layer from animation module plus target animator exists,valid kind; writes=delegated selector group/property creation,next_id; source=commands/animation.rs:add_selector;commands/text_anim.rs:add_selector,animator_uid",
    ),
    (
        "text.removeAllAnimators",
        "path=layers; resolves=layers_p explicit layers/layer else selected; reads=editor selected text eligibility,each target text/animators existence and content; holds=target animator children assigned empty; checks=has_text_layer,skip absent animator groups; writes=clear each text/animators.children; source=commands/animation.rs:remove_all_animators",
    ),
    (
        "layer.enablePerChar3D",
        "path=enable; resolves=layer_p,enabled supplied else toggle negates current perChar3d else true; reads=editor selected Text eligibility,source Text,perChar3d value/existence,existing animators properties/rotation membership/value types,names and shown_dims; holds=assigned perChar3d/three_d,existing affected animator field values and membership; checks=has_text_layer,target Text source; writes=create hidden static perChar3d if absent,set true,layer three_d true,Vec2 animator values expand to Vec3,shown_dims 3,add X/Y rotation when missing,rename Rotation Z Rotation,next_id; source=commands/text_anim.rs:enable_per_char,set_animators_3d",
    ),
    (
        "layer.enablePerChar3D",
        "path=disable; resolves=layer_p,enabled false or toggle from true; reads=perChar3d/source/animator members and types; holds=assigned perChar3d and affected animator shown_dims/names/removed rotationX/Y content; checks=has_text_layer,target Text; writes=perChar3d false,shown_dims 2,Vec2 expand still occurs,remove X/Y rotation,rename Z Rotation to Rotation,three_d switch retained,next_id; source=commands/text_anim.rs:enable_per_char,set_animators_3d",
    ),
    (
        "layer.applyTextPreset",
        "path=ordinary-preset; resolves=layer_p,preset id or case-insensitive name,start explicit finite comp seconds else editor active time; reads=text/perChar3d.value,layer start/stretch,animators group,project next_id; holds=Text/animators eligibility,perChar3d,assigned append location and layer timing; checks=has_text_layer,preset exists,finite start; writes=append fresh configured animators/selectors/keys,next_id,reply animator uids; source=commands/text_anim.rs:apply_preset,set_path;project/src/build.rs:text_animator,text_selector",
    ),
    (
        "layer.applyTextPreset",
        "path=ordinary-preset-frames; resolves=ordinary-preset plus positive frames,by explicit else preset-id Words/Lines/characters; reads=Source Text evaluated at start,font-independent word/line/character counts,comp frame_rate,preset last key span,layer timing and perChar3d; holds=sampled Source Text and timing/frame rate plus ordinary append eligibility; checks=frames finite positive; writes=ordinary preset key times scaled by frames*unit_count/fps/span; source=commands/text_anim.rs:frames_scale,apply_preset",
    ),
    (
        "layer.applyTextPreset",
        "path=blurSlide; resolves=layer_p,promo id/name,start explicit else time_of(comp),variant default/subtitle,by explicit else words unless CJK; reads=Text source,Source Text evaluated start and full layout unit counts,comp frame_rate,layer start/stretch,perChar3d,animators group,motionBlur default true; holds=Source Text/layout inputs,source/timing/perChar3d,assigned animator append and existing motion_blur/enable_motion_blur/shutter_angle/samples; checks=has_text_layer,valid variant/by/ease/nonnegative numbers; writes=append position/blur/opacity expression-selector animator,next_id,optional layer+comp motion blur settings,reply units/end; source=commands/promo_text.rs:Ctx::new,blur_slide;commands/promo_ease.rs:motion_blur",
    ),
    (
        "layer.applyTextPreset",
        "path=letterpress; resolves=promo layer/start,by explicit else characters; reads=Source Text layout and unit count,comp fps,layer timing/perChar3d,anchorGrouping/groupingAlignment keys existence,motionBlur; holds=layout/source/timing/perChar3d,assigned grouping fields and append placement,motion blur fields when enabled; checks=valid numeric/by/ease params; writes=anchorGrouping 0,groupingAlignment [0,-35] only when unkeyed,append scale/blur/opacity animator,next_id,optional motion blur; source=commands/promo_text.rs:letterpress,grouping;commands/promo_ease.rs:motion_blur",
    ),
    (
        "layer.applyTextPreset",
        "path=leadWordZoom; resolves=promo layer/start,by explicit else words when >1 else characters; reads=full Source Text layout/glyph positions/advances/line box/font metrics/doc size,layer timing/perChar3d,comp fps,groupingAlignment keys,motionBlur; holds=layout dependency closure,assigned grouping/append fields,motion blur fields; checks=nonempty leading glyphs,valid by/ease/nonnegative numbers; writes=grouping,three fresh shift/zoom/follow animators with sampled geometry and eased keys/expressions,next_id,optional motion blur,reply computed end; source=commands/promo_text.rs:lead_word_zoom,grouping;commands/promo_ease.rs:set_keys",
    ),
    (
        "layer.applyTextPreset",
        "path=typewriterFast-left-alignment; resolves=promo layer/start,frames default .7/cursor default vertical bar; reads=Source Text evaluated at start and layout,justify/line count/box_size,all Source Text static/key documents and anchor static/key values if centered or right single-line unboxed; holds=assigned source expr,all justification/anchor values changed by alignment,source text/layout for end; checks=valid nonnegative frames/cursor string; writes=Source Text expression,conditionally all static/key documents justify Left and shift all anchor values by layout-origin delta; source=commands/promo_text.rs:typewriter,left_align_in_place",
    ),
    (
        "layer.applyTextPreset",
        "path=countUp; resolves=promo layer/start,to explicit else first written number,from default 0,decimals explicit capped 6 else written decimals; reads=Source Text evaluated start including prefix/suffix/number,static Source Text document style for tabular figure mutation; holds=sampled source text,assigned source expr and static TextDoc figureWidth/base/runs; checks=number present unless to supplied,finite to,valid separator/nonnegative duration; writes=static TextDoc tabular figures and enabled count expression,reply from/to/end; source=commands/promo_text.rs:count_up,find_number;keyframe/src/text_doc.rs:set_attr",
    ),
    (
        "layer.applyTextPreset",
        "path=digitRoll; resolves=promo layer/start,digits from tabular-layout glyphs,cycles capped five,frames/stagger/fps; reads=Source Text entire layout/font/style,source layer name/start/in/out/stretch/three_d/perChar3d,comp width/height/duration/frame_rate/layer names and source stack position,animators and original static Source Text styles,motionBlur; holds=layout/style closure,source timing/3D/name,source stack place and naming inputs,assigned source figureWidth/animators and comp blur settings; checks=at least one ASCII digit,valid numeric/ease params; writes=two fresh parented strip/matte layers per digit inserted above source,complete timing/source/keys/matte content,hide-original-digits animator,tabular source,next_id,selected_layers source,optional strip+comp blur; source=commands/promo_text.rs:digit_roll;project/src/build.rs:layer;commands/promo_ease.rs:set_keys,motion_blur",
    ),
    (
        "text.animatorFontAxes",
        "path=list; resolves=layer_p,no axis; reads=Source Text evaluated layer_time(editor active time),layout first face else font resolve,font variation axes; holds=none(document write); checks=has_text_layer,TextDoc source value; writes=none(document),reply font/axes; source=commands/text_anim.rs:font_axes",
    ),
    (
        "text.animatorFontAxes",
        "path=existing-animator; resolves=layer_p,axis tag/name from actual font,explicit animator uid/index; reads=Source Text/font/layout at CTI,axis definitions,animator list/order/properties group; holds=font/axis eligibility,animator identity/properties insertion place; checks=has_text_layer,variable font with requested axis,existing properties group; writes=append fresh axis scalar property,next_id,reply animator/axis; source=commands/text_anim.rs:font_axes,animator_uid",
    ),
    (
        "text.animatorFontAxes",
        "path=new-animator; resolves=layer_p,axis tag/name,no animator param; reads=Source Text/font/layout/axes,text/animators children count; holds=font/axis eligibility,animators name count and append membership; checks=has_text_layer,known variable axis; writes=append fresh named animator with axis property/default selector,next_id; source=commands/text_anim.rs:font_axes;project/src/build.rs:text_animator",
    ),
    (
        "text.edit",
        "path=editor-session; resolves=comp_id,layer explicit else text_edit.layer else first selected Text; reads=target source Text,Source Text CTI length,editor text_edit/selected_layers; holds=none(document write); checks=editing_or_text_layer,target Text; writes=editor selected_layers and text_edit with clamped selection/caret/created flag,bump revision; source=commands/text_edit.rs:edit,target,layer_doc",
    ),
    (
        "text.endEdit",
        "path=no-empty-created-layer; resolves=editor text_edit; reads=text_edit.created,Source Text CTI text when created; holds=none(document write); checks=always; writes=take editor text_edit,bump revision; source=commands/text_edit.rs:end_edit,layer_doc",
    ),
    (
        "text.endEdit",
        "path=delete-empty-created-layer; resolves=active comp,text_edit.layer created true; reads=editor text_edit,Source Text evaluated CTI,layer start/stretch,comp layer membership; holds=target complete deleted layer plus empty-source eligibility; checks=created layer document empty; writes=remove target layer,remove from selected_layers,clear text_edit,bump; source=commands/text_edit.rs:end_edit",
    ),
    (
        "text.setSelection",
        "path=editor-session; resolves=target explicit/edited/selected Text,session_on starts caret at end if needed; reads=Source Text CTI length,editor text_edit/selected_layers; holds=none(document write); checks=editing_or_text_layer,valid start/end/anchor/caret or all; writes=editor clamped anchor/caret,clear pending when selection changes,goal_x none; source=commands/text_edit.rs:set_selection,session_on",
    ),
    (
        "text.moveCaret",
        "path=editor-layout-navigation; resolves=target edited Text,requested to and extend; reads=Source Text CTI full text/style/font/layout,editor text_edit caret/anchor/goal_x; holds=none(document write); checks=editing,known movement target; writes=editor caret/anchor/goal_x,pending none; source=commands/text_edit.rs:move_caret,move_target",
    ),
    (
        "text.insert",
        "path=text-replacement; resolves=target explicit/edited/first-selected Text,range explicit else session selection,new session defaults end,CTI active comp; reads=editor text_edit pending/range,time,Source Text value/keys/hold policy/full TextDoc runs/paragraphs,layer name/start/stretch; holds=assigned Source Text static value or CTI key and preserved interpolation,full document being modified,automatic-name eligibility and assigned name; checks=editing_or_text_layer,text supplied,Text source; writes=replace clamped char range with normalized newlines and pending/insertion style,Source Text value/key,auto layer name when allowed,editor caret after insert/pending and goal_x clear; source=commands/text_edit.rs:insert,edit_doc;keyframe/src/text_doc.rs:replace_range",
    ),
    (
        "text.delete",
        "path=nonempty-range; resolves=target,range explicit else selection else forward/back char/word from caret; reads=editor text_edit/range/time,Source Text full CTI document/runs/paragraphs,layer timing/name,word boundary text; holds=assigned Source Text value/key and document,auto-name eligibility/assigned name; checks=editing_or_text_layer; writes=delete char range via replace_range,Source Text value/key and possible layer rename,editor selection to range start/pending and goal_x clear; source=commands/text_edit.rs:delete,edit_doc;keyframe/src/text_doc.rs:replace_range",
    ),
    (
        "text.delete",
        "path=empty-range; resolves=target/session/range as deletion; reads=Source Text CTI length,editor caret/selection; holds=none(document write); checks=editing_or_text_layer; writes=none(document),may start editor text session; source=commands/text_edit.rs:delete,session_on",
    ),
    (
        "edit.pasteTextMatchFormatting",
        "path=replace-whole-layer; resolves=no text_edit,target explicit else selected Text,text supplied else text_clipboard.text; reads=clipboard when omitted,Source Text CTI full document/first style/paragraphs,layer name/timing; holds=Source Text assigned value/key,formatting retained,auto-name eligibility/name; checks=editing_or_text_layer,text available; writes=replace whole TextDoc text retaining first style/paragraphs,possible layer rename; source=commands/text_edit.rs:paste_match,edit_doc;keyframe/src/text_doc.rs:set_text",
    ),
    (
        "edit.pasteTextMatchFormatting",
        "path=edited-selection; resolves=target/session,text supplied else clipboard; reads=editor text_edit range/pending/time,Source Text CTI document/style at selection or insertion,layer timing/name; holds=assigned Source Text/key and retained style/paragraphs,name eligibility/assigned name; checks=editing_or_text_layer; writes=replace selection with explicit selected/insertion style,Source Text/key,auto name,editor caret/pending; source=commands/text_edit.rs:paste_match,edit_doc",
    ),
    (
        "edit.pasteTextFormattingOnly",
        "path=caret-only; resolves=editor text_edit empty selection; reads=text_clipboard first style/paragraph,editor text session; holds=none(document write); checks=has_text_clip; writes=editor text_edit.pending style only; source=commands/text_edit.rs:paste_formatting",
    ),
    (
        "edit.pasteTextFormattingOnly",
        "path=range-or-whole-layers; resolves=text_edit nonempty range else explicit layer else all selected Text layers; reads=editor selected_layers/text_edit/time,text_clipboard first style/paragraph,each Source Text CTI doc/styles/paragraph ranges,layer timing/name; holds=assigned Source Text value/key and full modified document,name eligibility; checks=has_text_clip,Text targets; writes=apply copied char style and paragraph attributes over range or whole doc,Source Text value/key; source=commands/text_edit.rs:paste_formatting,edit_doc;keyframe/src/text_doc.rs:apply_style,apply_para",
    ),
    (
        "text.fontFeatures",
        "path=font-query; resolves=font/style explicit else target edited/selected Text and style at selected range start or first character; reads=font database,optional Source Text CTI style/editor text_edit; holds=none(document write); checks=always; writes=none(document),reply features/options; source=commands/text_edit.rs:font_features",
    ),
    (
        "text.fonts",
        "path=font-query-or-rescan; resolves=query optional,rescan false default; reads=font database/system families/native names; holds=none(document write); checks=always; writes=font cache only when rescan,no document mutation; source=commands/text_edit.rs:fonts",
    ),
    (
        "anim.reveal",
        "path=frontend-reveal; resolves=comp_id,selected_layers else all comp layers,kind keyframes/animation/modified; reads=property keys/expr/value,Transform defaults from source/comp dimensions,effects/masks/text animator group membership; holds=none(document write); checks=has_comp; writes=frontend timeline.revealProps event only,reply revealed props; source=commands/animation.rs:reveal,modified;render/src/eval.rs:source_size",
    ),
    (
        "markers.set",
        "path=new-comp-marker; resolves=comp_id,layer omitted/null owner comp,new true; reads=comp frame_rate,editor active time even with explicit time,all markers ordering/equality for returned index; holds=frame snapping,marker insertion/order result dependencies,assigned supplied marker fields; checks=has_comp,valid label/cuePoint; writes=append default marker at snapped CTI overridden by supplied fields,sort times,reply first equal marker index; source=commands/markers.rs:set,owner,apply_fields",
    ),
    (
        "markers.set",
        "path=new-layer-marker; resolves=comp_id,layer id/index/name,new true; reads=layer markers_locked/start/stretch,comp frame_rate,editor active time,marker order/equality for result; holds=owner identity/marker-lock eligibility,layer timing,insert order/result dependencies; checks=has_comp,!markers_locked,valid fields; writes=append marker converting snapped comp time to layer time,sort,reply index; source=commands/markers.rs:set,owner,to_marker_time,apply_fields",
    ),
    (
        "markers.set",
        "path=existing-index; resolves=comp_id,optional layer owner,index zero-based; reads=owner marker list index membership,existing complete marker,layer markers_locked,comp frame_rate for supplied time/duration,layer timing for supplied layer time; holds=addressed marker index/time,complete retained-and-assigned marker fields,lock eligibility,sort/equality result dependencies; checks=has_comp,index in range,!layer markers_locked,valid fields; writes=remove original,apply supplied fields preserving others,append/sort,reply first equal marker index; source=commands/markers.rs:find_index,set,apply_fields",
    ),
    (
        "markers.set",
        "path=existing-at; resolves=owner,at comp seconds chooses first marker within half comp frame after layer-to-comp mapping; reads=all preceding marker times and target marker content,comp frame_rate,layer start/stretch/markers_locked; holds=at-address membership and matching ambiguity,assigned and retained marker fields,lock/timing; checks=has_comp,marker found,!markers_locked; writes=apply supplied fields and sort,reply index; source=commands/markers.rs:find_index,set,to_comp_time,apply_fields",
    ),
    (
        "markers.delete",
        "path=index; resolves=owner,index zero-based; reads=marker list indexed membership,layer markers_locked; holds=removed complete marker and index/time placement,lock eligibility; checks=has_comp,index exists,!markers_locked; writes=remove addressed marker; source=commands/markers.rs:delete,find_index",
    ),
    (
        "markers.delete",
        "path=at; resolves=owner,first marker within half comp frame of at in comp time; reads=marker times before target,comp frame_rate,layer start/stretch/markers_locked; holds=removed complete marker,at lookup ambiguity,lock/timing; checks=has_comp,found marker,!markers_locked; writes=remove addressed marker; source=commands/markers.rs:delete,find_index,to_comp_time",
    ),
    (
        "markers.convert",
        "path=layer-to-comp; resolves=layer owner,index or at lookup; reads=source complete marker/times,layer start/stretch,comp markers times/order; holds=source marker content/place,source timing,destination insertion order; checks=has_comp,marker exists,markers_locked not checked; writes=remove source layer marker,append clone to comp with converted time,sort comp markers; source=commands/markers.rs:convert,find_index,to_comp_time",
    ),
    (
        "markers.convert",
        "path=comp-to-layer; resolves=comp owner,index/at,toLayer explicit id/index/name else existing selected_layers; reads=editor selected_layers,source full marker/times,comp frame_rate for at,each target layer start/stretch/markers ordering; holds=source marker content/place,target identities/timing,destination insertion membership; checks=has_comp,nonempty target layers,markers_locked not checked; writes=remove comp marker,append clone at mapped layer time to each target,sort each; source=commands/markers.rs:convert,find_index",
    ),
    (
        "layer.updateMarkersFromSource",
        "path=precomp-targets; resolves=layers_p,only source Comp items with existing nested comp; reads=editor selected_layers/source eligibility,each layer source item/markers_locked/marker times,nested comp complete markers; holds=source Comp identity/nested marker content,destination lock/dedup time set; checks=has_precomp_layer,at least one precomp target,locked targets skipped; writes=append source markers absent by stored time,sort,reply added count; source=commands/markers.rs:update_from_source,has_precomp_layer",
    ),
    (
        "time.set",
        "path=frame; resolves=active comp,frame integer takes priority; reads=comp frame_rate/duration,project settings time_display/frame_start and comp display_start for report,prefs sync_time_related_items,nesting graph/source/timing and related comp frame_rate/duration if sync; holds=none(document write); checks=has_comp; writes=editor times active snapped/clamped and related times when preference on,reply time/frame/display; source=commands/time.rs:set,report,display_time;engine/src/lib.rs:set_time,sync_related_times",
    ),
    (
        "time.set",
        "path=timecode; resolves=active comp,timecode when frame absent; reads=editor active time for relative code,project time_display/frame_start,comp display_start/frame_rate/duration,sync preference and related graph/timing; holds=none(document write); checks=has_comp,valid timecode; writes=editor times snapped/clamped/synchronized,reply display; source=commands/time.rs:set,parse_display_time;engine/src/lib.rs:set_time",
    ),
    (
        "time.set",
        "path=seconds; resolves=active comp,time when frame/timecode absent; reads=comp frame_rate/duration,project display settings/comp display_start for report,sync preference and related graph/timing; holds=none(document write); checks=has_comp,time required; writes=editor times snapped/clamped/synchronized; source=commands/time.rs:set,report;engine/src/lib.rs:set_time",
    ),
    (
        "time.step",
        "path=editor-time; resolves=active comp,frames integer else one; reads=editor active time,comp frame_rate/duration,project display settings/display_start,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor times frame step then snap/clamp/sync; source=commands/time.rs:step,report;engine/src/lib.rs:set_time",
    ),
    (
        "time.nextFrame",
        "path=editor-time; resolves=active comp,step forced one frame; reads=editor active time,comp frame_rate/duration,report settings,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor time snapped/clamped/synchronized; source=commands/time.rs:specs,step;engine/src/lib.rs:set_time",
    ),
    (
        "time.previousFrame",
        "path=editor-time; resolves=active comp,step forced minus one frame; reads=editor active time,comp frame_rate/duration,report settings,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor time snapped/clamped/synchronized; source=commands/time.rs:specs,step;engine/src/lib.rs:set_time",
    ),
    (
        "time.forward10",
        "path=editor-time; resolves=active comp,step forced ten frames; reads=editor active time,comp frame_rate/duration,report settings,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor time snapped/clamped/synchronized; source=commands/time.rs:specs,step;engine/src/lib.rs:set_time",
    ),
    (
        "time.back10",
        "path=editor-time; resolves=active comp,step forced minus ten frames; reads=editor active time,comp frame_rate/duration,report settings,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor time snapped/clamped/synchronized; source=commands/time.rs:specs,step;engine/src/lib.rs:set_time",
    ),
    (
        "time.start",
        "path=editor-time; resolves=active comp,to start; reads=comp frame_rate/duration,report display settings,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor time zero then snap/clamp/sync; source=commands/time.rs:specs,go;engine/src/lib.rs:set_time",
    ),
    (
        "time.end",
        "path=editor-time; resolves=active comp,to end; reads=comp duration/frame_rate,report display settings,sync preference/related graph/timing; holds=none(document write); checks=has_comp; writes=editor time duration-frame_duration then snap/clamp/sync; source=commands/time.rs:specs,go;engine/src/lib.rs:set_time",
    ),
    (
        "time.layerIn",
        "path=editor-time; resolves=active comp,first selected layer else current time; reads=editor selected_layers/time,layer in_point,comp frame_rate/duration,report settings,sync related graph/timing; holds=none(document write); checks=has_comp; writes=editor time to in point snapped/clamped/synchronized; source=commands/time.rs:specs,go;engine/src/lib.rs:set_time",
    ),
    (
        "time.layerOut",
        "path=editor-time; resolves=active comp,first selected layer else current time; reads=editor selected_layers/time,layer out_point,comp frame_rate/duration,report settings,sync related graph/timing; holds=none(document write); checks=has_comp; writes=editor time to out-point minus frame snapped/clamped/synchronized; source=commands/time.rs:specs,go;engine/src/lib.rs:set_time",
    ),
    (
        "time.nextKey",
        "path=visible; resolves=active comp,visible literal layer/prop uids; reads=editor current time,comp markers/work_area/hide_shy/frame_rate/duration,all nonhidden layer markers/shy/timing,visible property key times,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor nearest time >CTI+half-frame then snap/clamp/sync; source=commands/time.rs:specs,go,step_to;engine/src/lib.rs:set_time",
    ),
    (
        "time.nextKey",
        "path=selected-or-all; resolves=active comp,selected layers else all,optional prop uid narrows and excludes markers/work area; reads=editor current time/selected_layers,property key times/layer timing,comp markers/work_area when no prop,frame_rate/duration,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor nearest later time then snap/clamp/sync; source=commands/time.rs:specs,go,step_to",
    ),
    (
        "time.previousKey",
        "path=visible; resolves=active comp,visible literal layer/prop uids; reads=editor current time,comp markers/work_area/hide_shy/frame_rate/duration,all nonhidden layer markers/shy/timing,visible property key times,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor nearest time <CTI-half-frame then snap/clamp/sync; source=commands/time.rs:specs,go,step_to",
    ),
    (
        "time.previousKey",
        "path=selected-or-all; resolves=active comp,selected layers else all,optional prop uid narrows and excludes markers/work area; reads=editor time/selected_layers,property key times/layer timing,comp markers/work_area when no prop,frame_rate/duration,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor nearest earlier time then snap/clamp/sync; source=commands/time.rs:specs,go,step_to",
    ),
    (
        "time.go",
        "path=start-end-work; resolves=active comp,to default start or end/workStart/workEnd; reads=comp duration/frame_rate/work_area according to target,editor current time,display report settings,sync preference/graph; holds=none(document write); checks=has_comp,valid to; writes=editor target time then snap/clamp/sync; source=commands/time.rs:go;engine/src/lib.rs:set_time",
    ),
    (
        "time.go",
        "path=layer-range; resolves=active comp,first selected layer for layerIn/layerOut; reads=editor selected_layers/time,layer in_point/out_point,comp frame_duration/duration,report settings,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor in or out-minus-frame else retains current,then snap/clamp/sync; source=commands/time.rs:go",
    ),
    (
        "time.go",
        "path=next-prev-visible; resolves=active comp,nextKey/prevKey with visible literal uid pairs; reads=CTI,visible property keys/timing,comp markers/work_area/hide_shy,nonhidden layer markers/shy/timing,frame_rate/duration,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor nearest time after/before half frame; source=commands/time.rs:go,step_to",
    ),
    (
        "time.go",
        "path=next-prev-selected-or-all; resolves=active comp,selected layers else all,optional prop uid; reads=editor CTI/selection,property key times/timing,comp markers/work_area and layer markers only when no prop,frame_rate/duration,sync preference/graph; holds=none(document write); checks=has_comp; writes=editor nearest time after/before half frame; source=commands/time.rs:go,step_to",
    ),
    (
        "file.importTimeline",
        "path=import; resolves=external path,explicit format else sniff bytes/path,edlFrameRate optional,probe rate from preferences; reads=external timeline/media service and importer,project next_id/default label preferences for new layers; holds=project allocation/append structure only,no existing project content needed by imported timeline mapping; checks=always,no unsupported native suffix,known format,document nesting <=256,parser succeeds; writes=new import folder/bins/footage/placeholder/solid/comp items and layers including timing/transforms/keys/markers/audio,next_id,editor project_selection/open comp/selection,event toast; source=commands/timeline.rs:import_timeline;formats/interchange/src/import.rs:import,Importer::run,fill_comp,clip_layer",
    ),
    (
        "file.exportTimeline",
        "path=native-export; resolves=comp_id,format explicit/path/default xml,prerender none or no rendered plans,precomps nest/prerender; reads=whole reachable comp graph and names/folders/source items/timing/switches/transforms/keys/audio/markers/source metadata for planning and serialization; holds=exported source graph and plan eligibility; checks=has_comp,valid format/prerender/precomps,path; writes=external interchange bytes,no document mutation,toast; source=commands/timeline.rs:export_timeline;formats/interchange/src/export.rs:export,plan,Exporter::sequence,motion,levels",
    ),
    (
        "file.exportTimeline",
        "path=prerender-export; resolves=comp_id,plan rendered layers in nested graph,output filenames from comp/layer names; reads=all native-export inputs plus complete render dependency closure/footage/expressions,cameras/lights,layer active ranges,comp dimensions/frame_rate/duration,prefs switches_affect_nested_comps,exporter supported formats; holds=entire rendered source closure and output-name/timing eligibility; checks=has_comp,ProRes exporter available per rendered layer,errors recorded as warnings; writes=external isolated-layer alpha movie files plus interchange bytes,no document mutation; source=commands/timeline.rs:export_timeline,prerender,isolate;formats/interchange/src/export.rs:plan_prerender",
    ),
    (
        "file.timelineFormats",
        "path=query; resolves=none; reads=static TimelineFormat list; holds=none(document write); checks=always; writes=none(document),reply formats; source=commands/timeline.rs:timeline_formats",
    ),
];

// Source-pinned after the prose rows were committed. P/S/T mean the active composition,
// generic layer/property/key/vertex selection and current times. Count a source read even
// when its result is discarded: time.rs:88-90 reads time/selection before the target match,
// prop.rs:107 reads time before reset/conversion, animation.rs:52 reads time before saving.
// Refinements of the prose: text_edit.rs:170 retains the old layer selection on empty-layer
// deletion; anim.rs:142 and time.rs:91 consume visible[].layer object IDs; link.rs:162 resolves
// target.comp through prop_ref. Post-comparison correction: engine/src/lib.rs:614-620 bypasses
// text.moveCaret's editing precondition with an explicit comp, so text_edit.rs:62 then reads
// selected_layers when no text_edit exists; the earlier PT reading missed that reachable path.
// Objects below exclude the target parameter and include alternate singular layer inputs
// accepted by layers_p (commands/mod.rs:535). Property/group UIDs are fields of a Layer.
#[rustfmt::skip]
const DECLARATION_EXPECTATIONS: &[(&str, &str, Option<&str>, &[(&str, &str)])] = &[
    ("prop.set", "PST", Some("layer"), &[("comp", "Item")]),
    ("prop.toggleAnimation", "PST", Some("layer"), &[("comp", "Item")]),
    ("prop.addKey", "PST", Some("layer"), &[("comp", "Item")]),
    ("prop.toggleKey", "PST", Some("layer"), &[("comp", "Item")]),
    ("keys.toggleTransform", "PST", Some("layers"), &[("comp", "Item"), ("layer", "Layer")]),
    ("prop.setExpression", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.reset", "PST", Some("layer"), &[("comp", "Item")]),
    ("prop.convertExpressionToKeyframes", "PST", Some("layer"), &[("comp", "Item")]),
    ("prop.select", "PST", Some("layer"), &[("comp", "Item")]),
    ("prop.deselect", "PST", Some("layer"), &[("comp", "Item")]),
    ("keys.select", "PS", None, &[("keys[].layer", "Layer")]),
    ("prop.renameGroup", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.setGroupEnabled", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.removeGroup", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.duplicateGroup", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.moveGroup", "PS", Some("layer"), &[("comp", "Item")]),
    ("keys.move", "PS", None, &[]),
    ("keys.delete", "PS", None, &[]),
    ("keys.easyEase", "PS", None, &[]),
    ("keys.easyEaseIn", "PS", None, &[]),
    ("keys.easyEaseOut", "PS", None, &[]),
    ("keys.toggleHold", "PS", None, &[]),
    ("keys.interpolation", "PS", None, &[]),
    ("keys.velocity", "PS", None, &[]),
    ("keys.copy", "PS", None, &[]),
    ("keys.paste", "PST", Some("layers"), &[("comp", "Item"), ("layer", "Layer")]),
    ("keys.selectAll", "PS", Some("layers"), &[("comp", "Item"), ("layer", "Layer"), ("visible[].layer", "Layer")]),
    ("keys.nudge", "PS", None, &[]),
    ("keys.nudgeForward", "PS", None, &[]),
    ("keys.nudgeBackward", "PS", None, &[]),
    ("keys.nudgeForward10", "PS", None, &[]),
    ("keys.nudgeBackward10", "PS", None, &[]),
    ("keys.set", "PS", Some("layer"), &[("comp", "Item")]),
    ("keys.setEase", "PS", Some("layer"), &[("comp", "Item")]),
    ("keys.selectEqual", "PS", None, &[]),
    ("keys.selectPrevious", "PS", None, &[]),
    ("keys.selectFollowing", "PS", None, &[]),
    ("keys.timeReverse", "PS", None, &[]),
    ("keys.wiggle", "PS", None, &[]),
    ("keys.smooth", "PS", None, &[]),
    ("motion.sketch", "PST", Some("layer"), &[("comp", "Item")]),
    ("anim.savePreset", "PST", None, &[]),
    ("anim.applyPreset", "PST", Some("layers"), &[("comp", "Item"), ("layer", "Layer")]),
    ("anim.addKeyframe", "PST", None, &[]),
    ("keys.exponentialScale", "PS", None, &[]),
    ("keys.audioToKeyframes", "PS", None, &[("comp", "Item")]),
    ("keys.rpfCameraImport", "PST", None, &[("comp", "Item")]),
    ("keys.transform", "PS", None, &[]),
    ("keys.setSpatialTangents", "PS", Some("layer"), &[("comp", "Item")]),
    ("keys.setLabel", "PS", None, &[]),
    ("keys.selectLabelGroup", "PS", None, &[]),
    ("keys.applyEase", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.separateDimensions", "PS", Some("layer"), &[("comp", "Item")]),
    ("prop.pickWhip", "PS", Some("layer"), &[("comp", "Item"), ("target.comp", "Item"), ("target.layer", "Layer")]),
    ("layer.addTextAnimator", "PS", Some("layer"), &[("comp", "Item")]),
    ("layer.addTextAnimatorProperty", "PS", Some("layer"), &[("comp", "Item")]),
    ("layer.addTextSelector", "PS", Some("layer"), &[("comp", "Item")]),
    ("layer.enablePerChar3D", "PS", Some("layer"), &[("comp", "Item")]),
    ("layer.applyTextPreset", "PST", Some("layer"), &[("comp", "Item")]),
    ("text.animatorFontAxes", "PST", Some("layer"), &[("comp", "Item")]),
    ("text.addSelector", "PS", Some("layer"), &[("comp", "Item")]),
    ("text.removeAllAnimators", "PS", Some("layers"), &[("comp", "Item"), ("layer", "Layer")]),
    ("text.edit", "PSTM", Some("layer"), &[("comp", "Item")]),
    ("text.endEdit", "PSTM", None, &[]),
    ("text.setSelection", "PSTM", Some("layer"), &[("comp", "Item")]),
    ("text.moveCaret", "PSTM", None, &[("comp", "Item"), ("layer", "Layer")]),
    ("text.insert", "PSTM", Some("layer"), &[("comp", "Item")]),
    ("text.delete", "PSTM", Some("layer"), &[("comp", "Item")]),
    ("edit.pasteTextMatchFormatting", "PSTM", None, &[("comp", "Item"), ("layer", "Layer")]),
    ("edit.pasteTextFormattingOnly", "PSTM", Some("layer"), &[("comp", "Item")]),
    ("text.fontFeatures", "PSTM", Some("layer"), &[("comp", "Item")]),
    ("text.fonts", "", None, &[]),
    ("anim.reveal", "PS", None, &[("comp", "Item")]),
    ("markers.set", "PT", Some("layer"), &[("comp", "Item")]),
    ("markers.delete", "P", Some("layer"), &[("comp", "Item")]),
    ("markers.convert", "PS", Some("layer"), &[("comp", "Item"), ("toLayer", "Layer")]),
    ("layer.updateMarkersFromSource", "PS", Some("layers"), &[("comp", "Item"), ("layer", "Layer")]),
    ("time.set", "PT", None, &[]),
    ("time.step", "PT", None, &[]),
    ("time.nextFrame", "PT", None, &[]),
    ("time.previousFrame", "PT", None, &[]),
    ("time.forward10", "PT", None, &[]),
    ("time.back10", "PT", None, &[]),
    ("time.start", "PST", None, &[]),
    ("time.end", "PST", None, &[]),
    ("time.layerIn", "PST", None, &[]),
    ("time.layerOut", "PST", None, &[]),
    ("time.nextKey", "PST", None, &[("visible[].layer", "Layer")]),
    ("time.previousKey", "PST", None, &[("visible[].layer", "Layer")]),
    ("time.go", "PST", None, &[("visible[].layer", "Layer")]),
    ("file.importTimeline", "", None, &[]),
    ("file.exportTimeline", "P", None, &[("comp", "Item")]),
    ("file.timelineFormats", "", None, &[]),
];

fn animation_scalar_reference_schema(node: &serde_json::Value) -> bool {
    match node.get("oneOf").and_then(serde_json::Value::as_array) {
        Some(branches) => !branches.is_empty() && branches.iter().all(animation_scalar_reference_schema),
        None => matches!(node.get("type").and_then(serde_json::Value::as_str), Some("integer" | "string")),
    }
}

#[test]
fn independent_animation_declarations_match_engine_reading() {
    let row_ids: BTreeSet<_> = ROWS.iter().map(|(id, _)| *id).collect();
    let expected_ids: BTreeSet<_> = DECLARATION_EXPECTATIONS.iter().map(|(id, ..)| *id).collect();
    assert_eq!(expected_ids.len(), DECLARATION_EXPECTATIONS.len(), "duplicate command expectations");
    assert_eq!(row_ids, expected_ids, "every committed command reading needs an exact comparison");
    let document = MotionDocument::create("Independent animation declarations", 64, 36).unwrap();
    let commands = document.commands();
    let mut failures = Vec::new();
    let mut totals = [0usize; 8];
    let mut write_totals = [0usize; 8];
    let mut write_count = 0usize;
    let mut differing_writes = Vec::new();
    let mut unsupported = Vec::new();
    for &(id, reads, target, objects) in DECLARATION_EXPECTATIONS {
        let command = commands.iter().find(|command| command.id == id).unwrap();
        write_count += usize::from(command.mutates);
        let failures_before = failures.len();
        let expected_reads: BTreeSet<_> = reads
            .chars()
            .map(|class| match class {
                'P' => "part".to_owned(),
                'S' => "selection".to_owned(),
                'T' => "time".to_owned(),
                // The text editing mode (C8-motion-read afdedb7b declares it).
                'M' => "mode".to_owned(),
                _ => panic!("invalid read class {class} for {id}"),
            })
            .collect();
        let actual_reads: BTreeSet<_> = command.reads.iter().map(|read| serde_json::to_value(read).unwrap().as_str().unwrap().to_owned()).collect();
        let expected_target: BTreeSet<_> = target.into_iter().map(str::to_owned).collect();
        let actual_target: BTreeSet<_> = command.targets.into_iter().map(str::to_owned).collect();
        // MotionDocument::execute refuses unknown top-level keys before running the handler
        // (wasm/src/lib.rs:518; kition-sdk schema.rs:957). Nested keys are not refused there.
        let accepted_objects: Vec<_> = objects
            .iter()
            .copied()
            .filter(|(name, _)| {
                let top = name.split(['.', '[']).next().unwrap();
                let accepted = command.schema.as_ref().and_then(|schema| schema.as_object()).is_some_and(|schema| schema.contains_key(top));
                if !accepted {
                    unsupported.push(format!("{id} write={} {name}", command.mutates));
                }
                accepted
            })
            .collect();
        let expected_objects: BTreeSet<_> = accepted_objects.iter().map(|(name, _)| (*name).to_owned()).collect();
        let actual_objects: BTreeSet<_> = command.objects.iter().map(|name| (*name).to_owned()).collect();
        let expected_types: BTreeSet<_> =
            accepted_objects.iter().map(|(name, ty)| format!("{name}:{ty}")).chain(target.into_iter().map(|name| format!("{name}:Layer"))).collect();
        let actual_types: BTreeSet<_> = command
            .object_types
            .iter()
            .flat_map(|(name, types)| types.iter().map(move |ty| (*name, *ty)))
            .filter(|(name, ty)| {
                let Some(parent) = name.strip_suffix("[]") else { return true };
                let array = command.schema.as_ref().and_then(|schema| schema.get(parent));
                // Identical per-item typing is redundant only for a declared scalar-ref list.
                // In particular, never collapse keys[].layer or target.comp nested roles.
                !(command.objects.contains(&parent)
                    && array.is_some_and(|node| node.get("type").and_then(serde_json::Value::as_str) == Some("array"))
                    && array.and_then(|node| node.get("items")).is_some_and(animation_scalar_reference_schema)
                    && command.object_types.iter().any(|(name, types)| *name == parent && types.contains(ty)))
            })
            .map(|(name, ty)| format!("{name}:{ty}"))
            .collect();
        let comparisons = [
            ("reads", expected_reads, actual_reads),
            ("targets", expected_target, actual_target),
            ("objects", expected_objects, actual_objects),
            ("object_types", expected_types, actual_types),
        ];
        let mut counts = Vec::new();
        for (index, (class, expected, actual)) in comparisons.into_iter().enumerate() {
            let less: Vec<_> = expected.difference(&actual).cloned().collect();
            let more: Vec<_> = actual.difference(&expected).cloned().collect();
            totals[index * 2] += less.len();
            totals[index * 2 + 1] += more.len();
            if command.mutates {
                write_totals[index * 2] += less.len();
                write_totals[index * 2 + 1] += more.len();
            }
            counts.push(format!("{class}:less={},more={}", less.len(), more.len()));
            if !less.is_empty() || !more.is_empty() {
                failures.push(format!("{id} {class}: less={less:?}; more={more:?}"));
            }
        }
        if command.mutates && failures.len() > failures_before {
            differing_writes.push(id);
        }
        println!("{id} write={} {}", command.mutates, counts.join("; "));
    }
    println!(
        "animation metadata {} commands [reads less/more, targets less/more, objects less/more, object_types less/more]: {totals:?}",
        DECLARATION_EXPECTATIONS.len()
    );
    println!(
        "animation metadata {write_count} registry writes [reads less/more, targets less/more, objects less/more, object_types less/more]: {write_totals:?}; differing writes={differing_writes:?}"
    );
    println!("animation handler branches with unsupported top-level parameters: {}: {}", unsupported.len(), unsupported.join("; "));
    let checked_fields: BTreeSet<_> = ANIMATION_EXACT_FIELD_COMMANDS.iter().copied().collect();
    let scoped_writes: BTreeSet<_> =
        commands.iter().filter(|command| command.mutates && row_ids.contains(command.id.as_str())).map(|command| command.id.as_str()).collect();
    assert_eq!(scoped_writes.len(), 75, "frozen scoped write-id count");
    assert!(checked_fields.is_subset(&scoped_writes));
    let unresolved: Vec<_> = scoped_writes.difference(&checked_fields).copied().collect();
    assert_eq!(unresolved.len(), 71, "frozen write IDs without any runtime field scenario");
    println!(
        "ANIMATION_FIELD_COVERAGE exact_branch_commands={} no_runtime_field_check={}; command-wide field equivalence remains unproved",
        checked_fields.len(),
        unresolved.len()
    );
    println!("ANIMATION_FIELD_UNRESOLVED {unresolved:?}");
    // These exact metadata checks do not compare the prose's held document field dependencies.
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn independent_animation_rows_have_registered_ids_and_unique_paths() {
    let mut seen = BTreeSet::new();
    for &(id, row) in ROWS {
        assert!(kition_motion_engine::commands::find(id).is_some(), "unregistered {id}");
        let path = row.split(';').next().unwrap().strip_prefix("path=").unwrap();
        assert!(!path.is_empty() && seen.insert((id, path)), "duplicate {id}/{path}");
        for field in ["resolves=", "reads=", "holds=", "checks=", "writes=", "source="] {
            assert!(row.contains(field), "missing {field} in {id}/{path}");
        }
    }
}

struct AnimationFieldReading {
    case: &'static str,
    command: &'static str,
    params: serde_json::Value,
    premises: Vec<kition_kit::Premise>,
    source: &'static str,
}

const ANIMATION_EXACT_FIELD_COMMANDS: &[&str] = &["prop.setExpression", "prop.pickWhip", "keys.setEase", "keys.set"];

fn animation_fields(id: u64, fields: &[&str], positions: &[&str]) -> kition_kit::Premise {
    kition_kit::Premise::on(serde_json::json!(id)).fields(fields.iter().copied()).positions(positions.iter().copied())
}

// Frozen from the handlers after the metadata comparison, before running the field comparison.
// These scenarios compare functional document fields and replaced fields, including writes
// supplied explicitly by the caller. A calculated value discarded on this branch is not a
// dependency. Object/property existence is assumed; key-list membership/windows are not compared.
// Numeric property UIDs avoid property-name/path resolution as an additional dependency.
// Interpretation caveat, retained without changing the frozen rows: key_timing's inPoint /
// outPoint reads affect only the reply warning, not the assigned key. This comparison includes
// response dependencies; a document-result-only interpretation would omit those two positions.
// Post-comparison selector correction: media/time/src/lib.rs:17 defines 254016000000 ticks/s.
// The original frozen selector literals used an incorrect 127008000000 ticks/s handoff value.
// Post-comparison path correction: kdm-model/src/fields.rs:253,269 puts node metadata in
// `.meta`; :297-331 puts key interpolation/ease/flags in `.data` (time/value stay direct).
fn source_animation_field_readings() -> (MotionDocument, Vec<AnimationFieldReading>) {
    use kition_kit::CallContext;
    use kition_motion_engine::Session;
    use kition_motion_project::{ItemId, LayerId};
    use serde_json::json;

    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Fields","width":320,"height":180,"frameRate":30,"duration":8})).unwrap()["comp"].as_u64().unwrap();
    let source = session.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
    let target = session.execute("layer.newSolid", json!({"name":"Target"})).unwrap()["layer"].as_u64().unwrap();
    let prop_uid = |layer, path| session.project.comp(ItemId(comp)).unwrap().layer(LayerId(layer)).unwrap().props.prop(path).unwrap().uid;
    let opacity = prop_uid(source, "transform/opacity");
    let scale = prop_uid(source, "transform/scale");
    let rotation = prop_uid(target, "transform/rotation");
    for (time, value) in [(0, [70, 80, 90]), (1, [100, 120, 140]), (2, [130, 170, 210])] {
        session.execute("prop.addKey", json!({"layer":source,"prop":scale,"time":time,"value":value})).unwrap();
    }
    let scale_property = session.project.comp(ItemId(comp)).unwrap().layer(LayerId(source)).unwrap().props.find(scale).unwrap();
    assert!(!scale_property.spatial);
    assert_eq!(scale_property.keys.len(), 3);
    assert_eq!(scale_property.keys.iter().map(|key| key.time.0).collect::<Vec<_>>(), vec![0, 254_016_000_000, 508_032_000_000]);
    assert!(scale_property.keys.iter().all(|key| !key.roving && !key.continuous && !key.auto_bezier && key.out_ease.is_empty()));
    let mut doc = MotionDocument::open("AnimationFields.kmotion", session.project.to_json().as_bytes()).unwrap();
    doc.set_context(&CallContext { part: Some(json!({"comp":comp})), time: Some(1.0), on: Vec::new() }).unwrap();

    // Updated at the kition-sdk re-pin (maintainer-approved): the middle key's roving flag is held
    // by the `~keyTimes` content premise (C8-motion-read 9acbfd94), not a field premise, and
    // In/Out only shape the reply warning (C8-effect-read 5c707f8d, the time5 policy).
    let key_set_fields = || {
        animation_fields(
            source,
            &[
                "attrs.stretch",
                "transform/scale.keys[time=0].data.roving",
                "transform/scale.keys[time=508032000000].data.roving",
                "transform/scale.keys[time=254016000000].time",
                "transform/scale.keys[time=254016000000].value",
            ],
            &["startTime"],
        )
    };
    let mut moved_fields = key_set_fields();
    moved_fields.fields.as_mut().unwrap().extend(["transform/scale.keys[time=0].time".to_owned(), "transform/scale.keys[time=508032000000].time".to_owned()]);
    let rows = vec![
        AnimationFieldReading {
            case: "explicit-expression-replaces-only-expr",
            command: "prop.setExpression",
            params: json!({"layer":source,"prop":opacity,"expression":"value + 1"}),
            premises: vec![animation_fields(source, &["transform/opacity.expr"], &[])],
            source: "commands/prop.rs:576 apply_expr,595 set_expr; explicit text discards the reference calculated at 624",
        },
        AnimationFieldReading {
            case: "default-expression-uses-own-reference",
            command: "prop.setExpression",
            params: json!({"layer":source,"prop":opacity,"enabled":true}),
            premises: vec![animation_fields(
                source,
                &["transform/opacity.expr", "transform.meta.match", "transform/opacity.meta.match", "transform/opacity.meta.name"],
                &[],
            )],
            source: "commands/prop.rs:576 apply_expr,624 defs; commands/link.rs:108 reference selects Transform by match and camel-cases leaf name",
        },
        AnimationFieldReading {
            case: "scalar-pick-whip-cross-layer",
            command: "prop.pickWhip",
            params: json!({"layer":source,"prop":opacity,"target":{"layer":target,"prop":rotation}}),
            premises: vec![
                animation_fields(source, &["transform/opacity.expr", "transform/opacity.value"], &[]),
                animation_fields(
                    target,
                    &["name", "transform.meta.match", "transform/rotation.meta.match", "transform/rotation.meta.name", "transform/rotation.value"],
                    &[],
                ),
            ],
            source: "commands/link.rs:108 reference,147 dims,154 pick_whip; scalar value avoids shown_dims/source/3D branch; commands/prop.rs:576 expression assignment",
        },
        AnimationFieldReading {
            case: "ease-out-preserves-unedited-linear-dimensions",
            command: "keys.setEase",
            params: json!({"layer":source,"prop":scale,"time":1,"side":"out","dim":0,"speed":12}),
            premises: vec![animation_fields(
                source,
                &[
                    "transform/scale.meta.spatial",
                    "transform/scale.keys[time=254016000000].time",
                    "transform/scale.keys[time=254016000000].value",
                    "transform/scale.keys[time=254016000000].data.out_interp",
                    "transform/scale.keys[time=254016000000].data.out_ease",
                    "transform/scale.keys[time=254016000000].data.auto_bezier",
                    "transform/scale.keys[time=254016000000].data.continuous",
                    "transform/scale.keys[time=508032000000].time",
                    "transform/scale.keys[time=508032000000].value",
                ],
                &[],
            )],
            source: "commands/anim.rs:252 set_ease; keyframe/lib.rs:391 side_ease linear nonspatial outgoing branch; empty ease list initializes all 3 dimensions from next-key slope; no retime_roving",
        },
        AnimationFieldReading {
            case: "key-value-and-time-uses-grid-neighbors-and-reply-range",
            command: "keys.set",
            params: json!({"layer":source,"prop":scale,"time":1,"newTime":1.6,"value":[105,125,145]}),
            premises: vec![moved_fields.clone(), animation_fields(comp, &["comp.attrs.frame_rate"], &[])],
            source: "commands/anim.rs:207 set_key; commands/prop.rs:147 key_time,195 key_timing; keyframe/lib.rs:431 retime_roving assigns endpoint flags and checks middle false flag before returning",
        },
        AnimationFieldReading {
            case: "key-value-only-has-no-grid-or-clamp-read",
            command: "keys.set",
            params: json!({"layer":source,"prop":scale,"time":1,"value":[105,125,145]}),
            premises: vec![key_set_fields()],
            source: "commands/anim.rs:207 set_key skips neighbor clamp without newTime; commands/prop.rs:147 key_time unsnapped layer time validates start/stretch,195 key_timing reads active range; keyframe/lib.rs:431 nonroving early return",
        },
        AnimationFieldReading {
            case: "uid-path-key-value-and-time",
            command: "keys.set",
            params: json!({"layer":source,"path":format!("@{scale}"),"time":1,"newTime":1.6,"value":[105,125,145]}),
            premises: vec![moved_fields, animation_fields(comp, &["comp.attrs.frame_rate"], &[])],
            source: "same set_key/time/retime dependencies as numeric prop case; commands/prop.rs:26 find_prop -> project/props.rs:379 prop accepts bare @uid and calls find, avoiding mutable name/match resolution",
        },
        AnimationFieldReading {
            case: "uid-path-key-value-only",
            command: "keys.set",
            params: json!({"layer":source,"path":format!("@{scale}"),"time":1,"value":[105,125,145]}),
            premises: vec![key_set_fields()],
            source: "same no-newTime set_key/time/retime dependencies as numeric prop case; commands/prop.rs:26 find_prop -> project/props.rs:379 prop accepts bare @uid and calls find",
        },
    ];
    (doc, rows)
}

// The pinned SDK 34dff06 guard.rs:352 records target ids and mentioned refs; 1729-1759
// replaces target defaults with Under premises; 1767-1801 gives other mentioned objects
// whole-object defaults only when a premise has not already named them. These fixtures use
// explicit numeric object ids, so no name/index/place resolution is modelled here.
fn animation_effective_needs(document: &MotionDocument, command: &str, params: &serde_json::Value) -> std::collections::BTreeMap<u64, kition_kit::Premise> {
    use kition_kit::Premise;
    use serde_json::json;

    let declaration = document.commands().into_iter().find(|declared| declared.id == command).unwrap();
    let references = |path: &str| {
        let mut values = vec![params];
        for segment in path.split('.') {
            let (name, array) = segment.strip_suffix("[]").map_or((segment, false), |name| (name, true));
            values = values
                .into_iter()
                .filter_map(|value| value.get(name))
                .flat_map(|value| if array { value.as_array().map(Vec::as_slice).unwrap_or_default().iter().collect::<Vec<_>>() } else { vec![value] })
                .collect();
        }
        values
            .into_iter()
            .flat_map(|value| value.as_array().map_or_else(|| vec![value], |array| array.iter().collect()))
            .map(|value| value.as_u64().unwrap())
            .collect::<Vec<_>>()
    };
    let mut needs = std::collections::BTreeMap::new();
    for id in declaration.targets.into_iter().flat_map(references) {
        let mut premise = Premise::on(json!(id));
        if declaration.position_independent {
            premise = premise.positions(Vec::<String>::new());
        }
        needs.insert(id, premise);
    }
    let under = document.under(command, params).unwrap().unwrap_or_default();
    assert!(under.objects.is_empty(), "{command}: explicit-id fixture unexpectedly resolved place objects {:?}", under.objects);
    for premise in under.premises {
        needs.insert(premise.id.as_u64().unwrap(), premise);
    }
    for id in declaration.objects.iter().flat_map(|path| references(path)) {
        needs.entry(id).or_insert_with(|| Premise::on(json!(id)));
    }
    needs
}

fn animation_path_covers(parent: &str, child: &str) -> bool {
    parent == child || child.strip_prefix(parent).is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('['))
}

// An ancestor is broader than the source reading, but covers its children. Report that as
// held-more, rather than also reporting each contained expected field as held-less.
fn animation_field_difference(expected: Option<&Option<Vec<String>>>, actual: Option<&Option<Vec<String>>>) -> (Vec<String>, Vec<String>) {
    let empty = Some(Vec::<String>::new());
    match (expected.unwrap_or(&empty), actual.unwrap_or(&empty)) {
        (None, None) => (Vec::new(), Vec::new()),
        (None, Some(_)) => (vec!["*".to_owned()], Vec::new()),
        (Some(_), None) => (Vec::new(), vec!["*".to_owned()]),
        (Some(expected), Some(actual)) => {
            let expected: BTreeSet<_> = expected.iter().collect();
            let actual: BTreeSet<_> = actual.iter().collect();
            let less = expected.iter().filter(|field| !actual.iter().any(|held| animation_path_covers(held, field))).map(|field| (*field).clone()).collect();
            let more =
                actual.iter().filter(|field| !expected.iter().any(|needed| animation_path_covers(needed, field))).map(|field| (*field).clone()).collect();
            (less, more)
        }
    }
}

#[test]
fn independent_animation_fields_match_source_scenarios() {
    use std::collections::BTreeMap;

    let (document, readings) = source_animation_field_readings();
    let checked_commands: BTreeSet<_> = readings.iter().map(|reading| reading.command).collect();
    assert_eq!(checked_commands, ANIMATION_EXACT_FIELD_COMMANDS.iter().copied().collect(), "frozen runtime field coverage ids");
    let saved = document.save(None).unwrap();
    let comp = document.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let mut differences = Vec::new();
    let mut totals = [0usize; 4];
    for reading in &readings {
        // Each frozen branch must also be reachable through the binding's accepted arguments.
        let mut executing = MotionDocument::open("AnimationFields.kmotion", &saved).unwrap();
        executing.set_context(&kition_kit::CallContext { part: Some(serde_json::json!({"comp":comp})), time: Some(1.0), on: Vec::new() }).unwrap();
        executing.execute(reading.command, reading.params.clone()).unwrap();
        let expected: BTreeMap<_, _> = reading.premises.iter().map(|premise| (premise.id.as_u64().unwrap(), premise)).collect();
        let actual = animation_effective_needs(&document, reading.command, &reading.params);
        let ids: BTreeSet<_> = expected.keys().chain(actual.keys()).copied().collect();
        for id in ids {
            let expected = expected.get(&id);
            let actual = actual.get(&id);
            for (index, class, wanted, held) in [
                (0, "fields", expected.map(|premise| &premise.fields), actual.map(|premise| &premise.fields)),
                (2, "positions", expected.map(|premise| &premise.positions), actual.map(|premise| &premise.positions)),
            ] {
                let (less, more) = animation_field_difference(wanted, held);
                totals[index] += less.len();
                totals[index + 1] += more.len();
                if !less.is_empty() || !more.is_empty() {
                    differences.push(format!("{}/{}/{id} {class}: less={less:?}, more={more:?}; source={}", reading.command, reading.case, reading.source));
                }
            }
        }
    }
    println!(
        "animation source fields: {} scenarios, fields less/more={}/{}, positions less/more={}/{}; list membership/windows excluded",
        readings.len(),
        totals[0],
        totals[1],
        totals[2],
        totals[3]
    );
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
