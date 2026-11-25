//! Independent command premises derived from engine handlers and their operations.

pub(crate) const ROWS: &[(&str, &str)] = &[
    (
        "mask.new",
        "path=append; resolves=comp,layer(id/name/position or selected-first); reads=editor.selection,time-independent,prefs.cycle_mask_colors,masks.children count,project.next_id; holds=layer exists,masks group exists; checks=nonempty vertices; writes=masks.children append,next_id,selected_vertices; source=commands/mask.rs:new_mask",
    ),
    (
        "mask.addVertex",
        "path=insert; resolves=comp,layer,mask(uid then ordinal/name or shape path uid); reads=editor.time,prefs.preserve_constant_vertex_count,layer.start_time/stretch,path.value_at(layer_time),path.keys,group.roto_bezier; holds=editable path; checks=point,index clamped; writes=path value/current key,other path keys if preserve,selected_vertices; source=commands/mask.rs:edit_path_counted,add_vertex,sync_keys",
    ),
    (
        "mask.setVertex",
        "path=replace; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,layer.start_time/stretch,path.value_at(layer_time),path.keys,group.roto_bezier; holds=editable path; checks=index exists; writes=path vertices/tangents value/current key,roto tangents; source=commands/mask.rs:edit_path_counted,set_vertex",
    ),
    (
        "mask.setClosed",
        "path=closed; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,layer.start_time/stretch,path.value_at(layer_time),path.keys,group.roto_bezier; holds=editable path; checks=path type; writes=path.closed value/current key,roto tangents; source=commands/mask.rs:edit_path_counted,set_closed",
    ),
    (
        "mask.selectVertices",
        "path=selection; resolves=active comp,vertices[].layer id/name/position; reads=editor.selected_vertices if add/toggle; holds=comp exists; checks=mask/index unsigned; writes=editor.selected_vertices; source=commands/mask.rs:vertex_groups,select_vertices",
    ),
    (
        "mask.moveVertices",
        "path=vertices; resolves=active comp,vertices[].layer or editor.selected_vertices layer/mask/index; reads=editor.selected_vertices,time,layer.start_time/stretch,path.value_at(layer_time),path.keys,group.roto_bezier; holds=enabled selected vertices; checks=delta,missing layers/groups/indices skipped; writes=path vertices value/current key,roto tangents; source=commands/mask.rs:edit_vertices_counted,move_vertices",
    ),
    (
        "mask.deleteVertices",
        "path=vertices; resolves=active comp,explicit vertices or editor.selected_vertices; reads=editor.time,prefs.preserve_constant_vertex_count,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag; holds=enabled selected vertices; checks=valid indices only; writes=path vertices/tangents/feather/closed current value/key,other keys if preserve,selected_vertices clear; source=commands/mask.rs:delete_vertices,edit_vertices_counted,sync_keys",
    ),
    (
        "mask.convertVertex",
        "path=tangents; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag,existing tangents for toggle,neighbor vertices/closed; holds=editable path; checks=index exists; writes=path tangents current value/key; source=commands/mask.rs:convert_vertex,edit_path_counted",
    ),
    (
        "mask.insertVertex",
        "path=split; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,prefs.preserve_constant_vertex_count,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag; holds=editable path; checks=segment exists including closed wrap; writes=path vertices/tangents/feather current value/key,other keys if preserve,selected_vertices; source=commands/mask.rs:insert_vertex,split_segment,edit_path_counted",
    ),
    (
        "mask.featherPoint.add",
        "path=feather; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag,geometry for nearest/inside-derived radius; holds=editable path; checks=segment or point,segment exists; writes=path.feather append current value/key,roto tangents; source=commands/mask.rs:feather_add,edit_path_counted",
    ),
    (
        "mask.featherPoint.set",
        "path=feather; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag,feather entry,geometry for nearest/toward/inside; holds=editable path; checks=feather index,segment exists; writes=path.feather fields current value/key,roto tangents; source=commands/mask.rs:feather_set,edit_path_counted",
    ),
    (
        "mask.featherPoint.remove",
        "path=feather; resolves=comp,layer,mask(uid/ordinal/name or shape path uid); reads=editor.time,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag,feather length; holds=editable path; checks=index exists unless all; writes=path.feather remove/clear current value/key,roto tangents; source=commands/mask.rs:feather_remove,edit_path_counted",
    ),
    (
        "mask.remove",
        "path=one; resolves=comp,layer,mask(uid then ordinal/name); reads=masks.children identities/order/names,editor.selected_vertices; holds=masks group; checks=mask exists; writes=masks.children remove,selected_vertices retain other layers; source=commands/mask.rs:remove_masks",
    ),
    (
        "mask.removeAll",
        "path=all; resolves=comp,layer; reads=masks.children,editor.selected_vertices; holds=masks group; checks=layer exists; writes=masks.children clear,selected_vertices retain other layers; source=commands/mask.rs:remove_masks",
    ),
    (
        "path.rotoBezier",
        "path=mask-options; resolves=selected-vertex masks else shape-path uid probe else explicit/selected/all masks via target_masks; reads=editor.selected_vertices/selected_props/selected_layers,mask kind.roto_bezier for all-on toggle,all static/key path geometry; holds=nonempty targets; checks=mask group exists under masks (shape-path target skipped); writes=mask.roto_bezier,path static/key tangents when on; source=commands/paths.rs:masks_for,roto_bezier,edit_mask_groups,map_path",
    ),
    (
        "path.convertToBezier",
        "path=mask-options; resolves=shape uid (a parametric shape item) else, with no mask target on a shape layer, every parametric shape, else selected-vertex masks else shape-path uid probe else explicit/selected/all masks; reads=editor.selected_vertices/selected_props/selected_layers,mask group identity,editor.time (the shapes' outline now); holds=nonempty targets; checks=mask group under masks only, or a parametric shape item; writes=mask.roto_bezier false, or the shape item replaced by a Path item; source=commands/paths.rs:masks_for,convert_to_bezier,edit_mask_groups,shape_convert.rs:convert",
    ),
    (
        "layer.mask.motionBlur",
        "path=mask-options; resolves=selected-vertex masks else shape-path uid probe else explicit/selected/all masks; reads=editor.selected_vertices/selected_props/selected_layers,mask kind; holds=nonempty targets; checks=mode enum,mask group under masks only; writes=mask.motion_blur; source=commands/paths.rs:mask_motion_blur,edit_mask_groups",
    ),
    (
        "layer.mask.featherFalloff",
        "path=mask-options; resolves=selected-vertex masks else shape-path uid probe else explicit/selected/all masks; reads=editor.selected_vertices/selected_props/selected_layers,mask kind; holds=nonempty targets; checks=mode enum,mask group under masks only; writes=mask.feather_falloff; source=commands/paths.rs:feather_falloff,edit_mask_groups",
    ),
    (
        "layer.mask.hideLocked",
        "path=editor; resolves=active comp; reads=editor.hide_locked_masks when value absent; holds=comp exists; checks=none; writes=editor.hide_locked_masks; source=commands/paths.rs:hide_locked",
    ),
    (
        "path.setFirstVertex",
        "path=rotate; resolves=active comp,explicit layer/mask uid/index or single selected vertex; reads=editor.selected_vertices,mask.path static and all keys,closed/vertex count; holds=one selected vertex enabled,mask exists; checks=static path closed,indices in range for rotation; writes=path static/key vertices/tangents order,selected_vertices index zero; source=commands/paths.rs:set_first_vertex,map_path",
    ),
    (
        "path.freeTransform",
        "path=points; resolves=selected-vertex masks else shape-path uid probe else explicit/selected/all masks; reads=editor.selected_vertices,time,layer.start_time/stretch,path.value_at(layer_time),path.keys,roto flag,selected vertex geometry or all geometry for default bounds anchor; holds=comp and layers; checks=nonidentity transform,editable path,valid indices only; writes=path vertices/tangents current value/key; source=commands/paths.rs:free_transform,masks_for",
    ),
    (
        "path.groupShapes",
        "path=group; resolves=active comp,explicit items on layer or selected shape items; reads=editor.selected_props/selected_layers,layer.source,contents tree/sibling order/group names,project.next_id; holds=selected shape items enabled; checks=items nonempty,same-parent items on first layer; writes=contents.children move into new group,next_id,selected_props; source=commands/paths.rs:selected_shape_items,group_shapes",
    ),
    (
        "path.ungroupShapes",
        "path=ungroup; resolves=active comp,explicit items on layer or selected shape items; reads=editor.selected_props/selected_layers,layer.source,contents tree/order,group.transform static/keys,inner positional static/keys; holds=selected shape items enabled; checks=group targets,static translation only (scale/rotation/skew/opacity rejected); writes=contents.children replace group by shifted inner items,inner positions/path vertices/gradient points/twist centers,selected_props; source=commands/paths.rs:ungroup_shapes,offset_items",
    ),
    (
        "mask.interpolate",
        "path=keys; resolves=comp,explicit layer or selected mask-key layer or selected-first,mask uid/ordinal/name or selected keys/props/vertices or sole mask; reads=editor.mask_interp/selected_keys/selected_props/selected_vertices,layer.start_time/stretch,comp.frame_rate,path.keys times/values; holds=mask path exists; checks=two existing keys,path values,options enums/rates; writes=path.keys inside span and endpoints,editor.mask_interp; source=commands/mask_interp.rs:interpolate,mask_uid",
    ),
    (
        "mask.interpolationOptions",
        "path=editor; resolves=none; reads=editor.mask_interp (partial merge and addVertices=true fallback); holds=always; checks=positive rate/vertices,unit/method enum; writes=editor.mask_interp; source=commands/mask_interp.rs:interp_options,apply_options",
    ),
    (
        "track.maskMethod",
        "path=editor; resolves=none; reads=none; holds=always; checks=method enum; writes=editor.mask_track_method; source=commands/mask_interp.rs:mask_method",
    ),
    (
        "track.mask",
        "path=analysis; resolves=comp,explicit layer or selected-mask layer or selected-first,mask uid/ordinal/name/selection/sole mask; reads=editor.mask_track_method/time/selection,layer.source/video flags/timing,comp.frame_rate/duration,mask.path value at first analysis layer-time,rendered source frames and transitive source render dependencies,face model,effects presence/source size,next_id,active track jobs; holds=selected mask enabled; checks=pixels,at least two frames/two vertices,method/direction,no running track/mask track; writes=mask.path keys and optional face-point effect/keys,next_id,editor.mask_track_method/job/time; source=commands/mask_interp.rs:track_mask,mask_track.rs:write_frames,start_mask_track,poll_mask_track_apply",
    ),
    (
        "track.extractFaceMeasurements",
        "path=measure; resolves=comp,explicit layer or selected layer with face points else first such layer; reads=editor.selection/time,layer.effects Face Track Points landmark keys/values,layer.start_time/stretch,comp.frame_duration,effects stack/names,next_id; holds=face points available; checks=effect exists; writes=Face Measurements effect and replacement measurement keys,next_id,key_clipboard/clip_is_keys; source=commands/mask_interp.rs:extract_face,mask_track.rs:face_effect",
    ),
    (
        "shape.dashes.add",
        "path=dashes; resolves=comp,layer,explicit stroke uid else selected containing stroke else first; reads=editor.selected_props,contents tree,stroke kind,dashes children/dash/gap values/order,next_id; holds=stroke exists; checks=dashes group,max three pairs; writes=dash static value or new dash/gap children,next_id; source=commands/shape_stroke.rs:stroke_ref,dashes_add",
    ),
    (
        "shape.dashes.remove",
        "path=dashes; resolves=comp,layer,explicit stroke uid else selected containing stroke else first; reads=editor.selected_props,contents tree,stroke kind,dashes children; holds=stroke exists; checks=dashes group; writes=last extra dash/gap children remove or dash.value zero and dash.keys clear; source=commands/shape_stroke.rs:stroke_ref,dashes_remove",
    ),
    (
        "shape.stroke.taper",
        "path=taper; resolves=comp,layer,explicit stroke uid else selected containing stroke else first; reads=editor.selected_props/time,contents tree,stroke kind,taper presence/target property ui and keys,layer.start_time/stretch,next_id; holds=stroke exists; checks=units enum or number,numeric fields; writes=taper group if missing and supplied fields value/current key,next_id; source=commands/shape_stroke.rs:stroke_ref,set_group,taper",
    ),
    (
        "shape.stroke.wave",
        "path=wave; resolves=comp,layer,explicit stroke uid else selected containing stroke else first; reads=editor.selected_props/time,contents tree,stroke kind,wave presence/target property ui and keys,layer.start_time/stretch,next_id; holds=stroke exists; checks=units enum or number,numeric fields; writes=wave group if missing and supplied fields value/current key,next_id; source=commands/shape_stroke.rs:stroke_ref,set_group,wave",
    ),
    (
        "shape.fillStroke",
        "path=paint; resolves=comp,layers/selection filtered shape source; reads=editor.time/selection,layer.source/start_time/stretch,contents tree/path/stroke/fill presence/order,paint property keys,first paint for reply,next_id; holds=at least one shape layer; checks=finite nonnegative width; writes=fill/stroke color and stroke/gstroke width value/current key,missing strokes if requested,next_id; source=commands/shape_stroke.rs:fill_stroke,paint,first_paint",
    ),
    (
        "shape.fillStroke",
        "path=report; resolves=comp,layers/selection filtered shape source; reads=editor.time/selection,layer.source/start_time/stretch,contents first fill/stroke values/keys; holds=shape layer; checks=no valid paint values supplied; writes=none (journal command); source=commands/shape_stroke.rs:fill_stroke,first_paint",
    ),
    (
        "shape.newShape",
        "path=existing-comp-space; resolves=comp,explicit shape layer or first selected unlocked shape; reads=editor.shape_tool/time/selection,layer.source/locked,comp.width/height,layer and ancestor transforms/time mappings,contents tree names,next_id; holds=comp exists; checks=kind,size,paint enums/colors,shape source,invertible transform; writes=contents new group/path/paint/transform,next_id,selected_layers; source=commands/shape_tool.rs:new_shape,draw_target,add_to_contents",
    ),
    (
        "shape.newShape",
        "path=existing-layer-space; resolves=comp,explicit shape layer or first selected unlocked shape; reads=editor.shape_tool/selection,layer.source/locked,comp.width/height default position,contents names,next_id; holds=comp exists; checks=kind,size,paint,shape source; writes=contents new group/path/paint/transform,next_id,selected_layers; source=commands/shape_tool.rs:new_shape,draw_target,add_to_contents",
    ),
    (
        "shape.newShape",
        "path=new-layer; resolves=comp,no existing shape target; reads=editor.shape_tool/selected_layers,selected layer insertion position,layer sources/counts,comp dimensions/duration/frame_rate,next_id; holds=comp exists; checks=kind,size,paint; writes=new layer and contents,next_id,selected_layers; source=commands/shape_tool.rs:new_shape,new_shape_layer,layer.rs:insert_layer",
    ),
    (
        "shape.toolOptions",
        "path=editor; resolves=none; reads=editor.shape_tool unless reset; holds=always; checks=paint colors/types/blends/gradient stops/finite numeric values; writes=editor.shape_tool; source=commands/shape_tool.rs:tool_options,paint_p",
    ),
    (
        "path.pointsFollowNulls",
        "path=path; resolves=comp,explicit path/prop or selected first path or first visible path on layer/selected-first; reads=editor.selected_props/selected_layers/time,source path evaluated value and expression dependencies,layer/ancestor transforms/time mappings,layer/property names and reference path,comp layer names/order/dimensions/duration,next_id,path.expression; holds=path type,nonempty vertices,reference constructible; checks=target exists; writes=new null layers/position values,source path.expression,selected_layers,next_id; source=commands/path_nulls.rs:target,info,points_follow_nulls,new_null",
    ),
    (
        "path.pointsFollowNulls",
        "path=pins; resolves=comp,pins uid/name or selected pins or all layer pins when no path; reads=editor.selection/time,layer puppet tree/pin kinds/names/position evaluated values and dependencies,layer/ancestor transforms,reference paths,comp layer names/order/dimensions/duration,next_id,pin.position.expression; holds=moving pins with positions; checks=no explicit path,pin resolution; writes=new null layers/position values,pin.position.expression,selected_layers,next_id; source=commands/path_nulls.rs:pin_targets,points_follow_nulls,new_null",
    ),
    (
        "path.nullsFollowPoints",
        "path=path; resolves=comp,explicit path/prop or selected first path or first visible path on layer/selected-first; reads=editor.selected_props/selected_layers/time,path evaluated value/dependencies,layer/ancestor transforms,time mappings,layer/property names/reference path,comp layer names/order/dimensions/duration,next_id; holds=path type,nonempty vertices,reference constructible; checks=target exists; writes=new null layers/positions/position expressions,selected_layers,next_id; source=commands/path_nulls.rs:target,info,nulls_follow_points,new_null",
    ),
    (
        "path.nullsFollowPoints",
        "path=pins; resolves=comp,pins uid/name or selected pins or all layer pins when no path; reads=editor.selection/time,pin kinds/names/position evaluated values/dependencies,layer/ancestor transforms/reference paths,comp layer names/order/dimensions/duration,next_id; holds=moving pins with positions; checks=no explicit path,pin resolution; writes=new null layers/position values/expressions,selected_layers,next_id; source=commands/path_nulls.rs:pin_targets,nulls_follow_points,new_null",
    ),
    (
        "path.tracePath",
        "path=path; resolves=comp,explicit path/prop or selected first path or first visible path on layer/selected-first; reads=editor.selected_props/selected_layers/time,path evaluated value/dependencies,layer/ancestor transforms,time mappings,layer/property names/reference path,layer.in_point/out_point,comp layer names/order/dimensions/duration,next_id; holds=path type,nonempty vertices,reference constructible; checks=target exists; writes=new null/position expression,Progress effect/slider keys/optional expression,selected_layers,next_id; source=commands/path_nulls.rs:target,info,trace_path,new_null",
    ),
    // Pinned from the declaration after review: later families followed guard-summary exposure.
    // Only the preceding 43 rows (37 registry writes) were committed fully blind in 28c89303.
    (
        "layer.newCamera",
        "path=create; resolves=comp; reads=editor.time/selection insertion,comp dimensions/duration/frame_rate,next_id; holds=comp exists; checks=camera type/preset; writes=new camera layer/properties,next_id,selected_layers; source=commands/three_d.rs:new_camera,apply_camera,layer.rs:insert_layer",
    ),
    (
        "layer.cameraSettings",
        "path=settings; resolves=comp,layer or selected-first; reads=editor.time,comp.width/frame_rate,layer.source/auto_orient/start_time/stretch,target property keys,current zoom for fStop,current position/orientation/poi/zoom for type conversion; holds=camera source; checks=type/preset; writes=name,supplied camera/transform fields value/current key,focusDistance with zoom lock,orientation/poi/auto_orient on type conversion; source=commands/three_d.rs:camera_settings,apply_camera",
    ),
    (
        "layer.newLight",
        "path=create; resolves=comp; reads=editor.time/selection insertion,comp dimensions/duration/frame_rate,next_id; holds=comp exists; checks=light kind/falloff; writes=new light layer/properties,next_id,selected_layers; source=commands/three_d.rs:new_light,apply_light,layer.rs:insert_layer",
    ),
    (
        "layer.lightSettings",
        "path=same-kind; resolves=comp,layer or selected-first; reads=editor.time,comp.frame_rate,layer.source/start_time/stretch,target property keys; holds=light source; checks=kind/falloff; writes=name,supplied light/position/poi fields value/current key; source=commands/three_d.rs:light_settings,apply_light",
    ),
    (
        "layer.lightSettings",
        "path=change-kind; resolves=comp,layer or selected-first; reads=editor.time,comp dimensions/duration/frame_rate,layer source/whole cloned layer,shared transform/light static values/keys/expressions,next_id; holds=light source; checks=kind/falloff; writes=source,auto_orient,whole rebuilt props preserving shared values/keys/expressions,supplied fields,name,next_id; source=commands/three_d.rs:light_settings,apply_light",
    ),
    (
        "layer.autoOrient",
        "path=orient; resolves=comp,layers or selection; reads=editor.selection,layer.source,transform.poi presence; holds=nonempty layers; checks=mode,rig cannot face camera,poi mode requires rig/poi; writes=layer.auto_orient; source=commands/three_d.rs:auto_orient",
    ),
    (
        "view.set3DView",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=view enum; writes=views3d.current/last; source=commands/three_d.rs:set_3d_view,switch_view",
    ),
    (
        "view.threeD.activeCamera",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.default",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.front",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.left",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.top",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.back",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.right",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.bottom",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.custom1",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.custom2",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.custom3",
        "path=editor; resolves=comp; reads=editor.views3d current; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:view_cmd,switch_view",
    ),
    (
        "view.threeD.last",
        "path=editor; resolves=comp; reads=editor.views3d current/last; holds=comp; checks=none; writes=views3d.current/last; source=commands/three_d.rs:last_view,switch_view",
    ),
    (
        "view.reset3DView",
        "path=custom-view; resolves=comp; reads=editor.views3d.current; holds=comp; checks=view is not activeCamera; writes=views3d.cams remove current; source=commands/three_d.rs:reset_view",
    ),
    (
        "view.reset3DView",
        "path=active-camera; resolves=comp,first active camera by source/video/timing/order; reads=editor.views3d.current/time,comp.width/height,layer.start_time/stretch,camera zoom value/keys,position/poi/orientation keys; holds=active camera exists; checks=current activeCamera; writes=camera position/poi/orientation value/current key; source=commands/three_d.rs:reset_view",
    ),
    (
        "view.set3DViewCamera",
        "path=custom-view; resolves=comp,view enum; reads=editor.views3d camera entry or defaults,comp.width/height; holds=comp; checks=view not activeCamera; writes=views3d.cams eye/poi/zoom; source=commands/three_d.rs:set_view_camera",
    ),
    (
        "camera.orbit",
        "path=custom-view; resolves=comp; reads=editor.views3d.current/camera,comp.width/height; holds=comp; checks=current not activeCamera; writes=views3d.cams eye/poi/zoom; source=commands/three_d.rs:camera_tool,Rig::orbit",
    ),
    (
        "camera.orbit",
        "path=active-camera; resolves=comp,explicit camera layer or first active camera; reads=editor.views3d.current/time,comp dimensions,source/video/timing/order for active camera,layer.start_time/stretch/auto_orient,position/poi/orientation/rotationX/rotationY/zoom values/keys; holds=camera exists; checks=source camera; writes=position and poi for two-node or orientation/rotationX/rotationY for one-node value/current key; source=commands/three_d.rs:camera_tool,Rig::orbit",
    ),
    (
        "camera.pan",
        "path=custom-view; resolves=comp; reads=editor.views3d.current/camera,comp.width/height; holds=comp; checks=current not activeCamera; writes=views3d.cams eye/poi/zoom; source=commands/three_d.rs:camera_tool,Rig::pan",
    ),
    (
        "camera.pan",
        "path=active-camera; resolves=comp,explicit camera layer or first active camera; reads=editor.views3d.current/time,comp dimensions,source/video/timing/order for active camera,layer.start_time/stretch/auto_orient,position/poi/orientation/rotationX/rotationY/zoom values/keys; holds=camera exists; checks=source camera; writes=position and poi for two-node or orientation/rotationX/rotationY for one-node value/current key; source=commands/three_d.rs:camera_tool,Rig::pan",
    ),
    (
        "camera.dolly",
        "path=custom-view; resolves=comp; reads=editor.views3d.current/camera,comp.width/height; holds=comp; checks=current not activeCamera; writes=views3d.cams eye/poi/zoom; source=commands/three_d.rs:camera_tool,Rig::dolly",
    ),
    (
        "camera.dolly",
        "path=active-camera; resolves=comp,explicit camera layer or first active camera; reads=editor.views3d.current/time,comp dimensions,source/video/timing/order for active camera,layer.start_time/stretch/auto_orient,position/poi/orientation/rotationX/rotationY/zoom values/keys; holds=camera exists; checks=source camera; writes=position and poi for two-node or orientation/rotationX/rotationY for one-node value/current key; source=commands/three_d.rs:camera_tool,Rig::dolly",
    ),
    (
        "view.lookAtSelected",
        "path=2d; resolves=comp,layers/selection; reads=editor.time,layer source/content bounds/evaluated transform and ancestor dependencies,comp.has_3d; holds=nonempty targets; checks=no 3d; writes=frontend view.lookAt event rectangle; source=commands/three_d.rs:look_at,layer_points",
    ),
    (
        "view.lookAtSelected",
        "path=custom-view; resolves=comp,layers/selection; reads=editor.time/views3d,layer content bounds/evaluated transform and ancestor dependencies,comp dimensions/has_3d; holds=nonempty targets; checks=3d,current not activeCamera; writes=views3d.cams eye/poi/zoom; source=commands/three_d.rs:look_at,layer_points",
    ),
    (
        "view.lookAtSelected",
        "path=active-camera; resolves=comp,layers/selection,first active camera; reads=editor.time/views3d,layer content bounds/evaluated transforms and ancestors,comp dimensions/has_3d,camera source/video/timing/order/auto_orient/position/poi/orientation/zoom and keys/time mapping; holds=nonempty targets,camera; checks=3d,current activeCamera; writes=camera position/poi value/current key (handler maps layer time twice); source=commands/three_d.rs:look_at,layer_points",
    ),
    (
        "view.lookAtAll",
        "path=2d; resolves=comp,active noncamera/nonlight layers by source/switches/timing; reads=editor.time,layer content bounds/evaluated transforms and ancestor dependencies,comp.has_3d; holds=nonempty visible targets; checks=no 3d; writes=frontend view.lookAt event rectangle; source=commands/three_d.rs:look_at,layer_points",
    ),
    (
        "view.lookAtAll",
        "path=custom-view; resolves=comp,active noncamera/nonlight layers by source/switches/timing; reads=editor.time/views3d,layer content bounds/evaluated transforms and ancestor dependencies,comp dimensions/has_3d; holds=nonempty targets; checks=3d,current not activeCamera; writes=views3d.cams eye/poi/zoom; source=commands/three_d.rs:look_at,layer_points",
    ),
    (
        "view.lookAtAll",
        "path=active-camera; resolves=comp,active noncamera/nonlight layers,first active camera; reads=editor.time/views3d,layer content bounds/evaluated transforms and ancestors,comp dimensions/has_3d,camera source/video/timing/order/auto_orient/position/poi/orientation/zoom and keys/time mapping; holds=nonempty targets,camera; checks=3d,current activeCamera; writes=camera position/poi value/current key (handler maps layer time twice); source=commands/three_d.rs:look_at,layer_points",
    ),
    (
        "camera.fromView",
        "path=create; resolves=comp,current custom view; reads=editor.views3d/time/selection insertion,comp dimensions/duration/frame_rate/layer names,next_id; holds=comp; checks=current not activeCamera; writes=new camera layer/position/poi/zoom/focusDistance,next_id,selection,views3d.current/last; source=commands/three_d.rs:camera_from_view",
    ),
    (
        "layer.newModel",
        "path=existing-item; resolves=comp,item id/name or project_selection first; reads=editor.project_selection/selection insertion,item.name/kind/footage metadata and model bytes bounds/animation names,comp dimensions/frame_rate/duration/renderer,all layer source/geometry presence/props insertion order,next_id; holds=model footage item; checks=index,position; writes=new model layer/geometry/timing/position,comp.renderer,missing geometry groups for all text/shape layers,next_id,selected_layers; source=commands/model3d.rs:new_model,use_advanced_3d,sync_geometry_options",
    ),
    (
        "layer.newModel",
        "path=import; resolves=comp,path through file.import; reads=host lent model file bytes,import settings/project items,comp dimensions/frame_rate/duration/renderer,all layer source/geometry presence/order,editor.selection insertion,next_id; holds=import succeeds/model footage; checks=index,position; writes=imported item,new model layer,comp.renderer,missing geometry groups for text/shape layers,next_id,selection; source=commands/model3d.rs:new_model,file.rs:import,use_advanced_3d",
    ),
    (
        "layer.new3dPrimitive",
        "path=create; resolves=comp; reads=editor.time/selection insertion,comp dimensions/duration/frame_rate/renderer,all layer source/geometry presence/order,next_id; holds=comp; checks=primitive kind,material options availability; writes=new primitive layer/geometry/material,comp.renderer,missing geometry groups for all text/shape layers,next_id,selection; source=commands/model3d.rs:new_primitive,use_advanced_3d",
    ),
    (
        "layer.newPrimitive",
        "path=create; resolves=comp; reads=editor.time/selection insertion,comp dimensions/duration/frame_rate/renderer,all layer source/geometry presence/order,next_id; holds=comp; checks=primitive kind,material options availability; writes=new primitive layer/geometry/material,comp.renderer,missing geometry groups for all text/shape layers,next_id,selection; source=commands/model3d.rs:new_primitive,use_advanced_3d",
    ),
    (
        "comp.renderer",
        "path=report; resolves=comp; reads=comp.renderer; holds=comp; checks=no renderer/value supplied; writes=none (journal command); source=commands/model3d.rs:comp_renderer",
    ),
    (
        "comp.renderer",
        "path=set; resolves=comp; reads=all layers.source/geometry group presence and child order,next_id,model source existence for toast; holds=comp; checks=renderer enum; writes=comp.renderer,missing geometryOptions on text/shape layers if advanced,next_id; source=commands/model3d.rs:comp_renderer,sync_geometry_options",
    ),
    (
        "layer.environment",
        "path=set; resolves=comp,layers/selection; reads=layer.source,first environment flag if on absent; holds=nonempty AV nonmodel layers; checks=all sources eligible; writes=environment,three_d true if on; source=commands/model3d.rs:environment",
    ),
    (
        "material.set",
        "path=fields; resolves=comp,layers/selection; reads=editor.time,comp.frame_rate,layer.start_time/stretch,material property availability/keys; holds=nonempty layers; checks=shadow enum,at least one matched option and each supplied option available somewhere; writes=supplied material values/current keys; source=commands/model3d.rs:material_set,apply_material",
    ),
    (
        "material.reset",
        "path=reset; resolves=comp,layers/selection; reads=layer.source,material group position/presence,next_id; holds=nonempty layers; checks=eligible source or skip; writes=replacement materialOptions subtree,next_id; source=commands/model3d.rs:material_reset,default_material,replace_material",
    ),
    (
        "material.duplicateAssign",
        "path=copy; resolves=comp,layers/selection first source then destinations; reads=source materialOptions static/keys/expressions,destination material property match ids and group structure; holds=two layers,source material exists; checks=dest material missing skipped; writes=shared destination material values/keys/expressions; source=commands/model3d.rs:material_duplicate",
    ),
    (
        "material.revealSource",
        "path=editor; resolves=comp,layer/selected-first; reads=layer.source Model item reference; holds=model source; checks=source kind; writes=project_selection; source=commands/model3d.rs:material_reveal",
    ),
    (
        "camera.stereoRig",
        "path=rig; resolves=comp,first active camera or create master; reads=editor.time/selection insertion,active-camera source/video/timing/order,master evaluated transform/camera/auto_orient and ancestors/expression dependencies,comp all settings/item name/parent,project item/layer names,next_id; holds=comp; checks=stereo enums/effect availability; writes=optional master,new controls null/effect,left/right/output comp items and layers/camera expressions/Glasses effect,next_id,project_selection; source=commands/rig3d.rs:stereo_rig,focus_distance,eye_exprs",
    ),
    (
        "camera.orbitNull",
        "path=rig; resolves=comp,layer/camera ref or selected camera or active camera; reads=editor.selection/time,camera.source/parent/name,active-camera source/video/timing/order,camera evaluated frame/focus and ancestors/expression dependencies,position/poi static/all keys,comp dimensions/duration/layer names/order,next_id; holds=camera no parent; checks=camera source; writes=new null,camera.parent and position/poi static/all keys,next_id,selected_layers; source=commands/rig3d.rs:camera_p,orbit_null",
    ),
    (
        "light.controlWithCamera",
        "path=attach; resolves=comp,lights explicit or selected light sources,camera explicit or active; reads=editor.selection/time,light.source/parent (toggle),active-camera source/video/timing/order,light transform property presence,comp dimensions; holds=lights and camera; checks=all lights valid; writes=light.parent,auto_orient,position/poi/orientation/rotations static values clearing keys/expressions; source=commands/rig3d.rs:control_light,set",
    ),
    (
        "light.controlWithCamera",
        "path=release; resolves=comp,lights explicit or selected,camera explicit or active; reads=editor.selection/time,light.source/parent,evaluated world frame and ancestor/expression dependencies,active camera selection dependencies,comp dimensions; holds=lights and camera; checks=release only when parent equals camera; writes=light.parent none,position/poi static clearing keys/expressions,auto_orient if poi present; source=commands/rig3d.rs:control_light,set",
    ),
    (
        "camera.fromModel",
        "path=models; resolves=comp,explicit model layers or selected model sources; reads=editor.selection/time,layer Model item,footage model bytes camera hierarchy/projection,geometry.unitScale evaluated value,world transform/ancestors/expression dependencies,comp dimensions/duration/layer names/insertion,next_id; holds=model sources readable with cameras; checks=found cameras; writes=new camera layers/options/transforms,next_id,selection; source=commands/rig3d.rs:model_layers,from_model",
    ),
    (
        "light.fromModel",
        "path=models; resolves=comp,explicit model layers or selected model sources; reads=editor.selection/time,layer Model item,footage model bytes light hierarchy/kind/options,geometry.unitScale evaluated value,world transform/ancestors/expression dependencies,comp dimensions/duration/layer names/insertion,next_id; holds=model sources readable with lights; checks=found lights; writes=new light layers/options/transforms,next_id,selection; source=commands/rig3d.rs:model_layers,from_model",
    ),
    (
        "light.environmentBackground",
        "path=background; resolves=comp,explicit environment light or selected/first environment light,source property layer id or first environment layer; reads=editor.selection/time,light source kind/source property keys and layer-time mapping,environment flags/order,source layer.name/source,referenced item/source size,comp dimensions/duration/layer names,next_id; holds=environment source footage/comp/solid; checks=source exists and kind eligible; writes=new source-sharing backdrop layer three_d/environment_background,next_id,selected_layers; source=commands/rig3d.rs:environment_background",
    ),
    (
        "track.camera",
        "path=analyze; resolves=comp,layer/selected-first,first existing camera tracker or add; reads=editor.selection,layer.source/adjustment/timing,comp.frame_rate/duration,source-frame render dependencies/effects preceding tracker,tracker static settings/tracks/deleted/signatures,effect bounds/source item,next_id,camera job state; holds=trackable layer enabled; checks=shot/method/aov,not adjustment,two frames,no running camera analysis; writes=optional tracker/settings static values clearing keys,analysis tracks/solve/signatures/method/error,selected_props,job,next_id; source=commands/camera_cmds.rs:track_camera,camera_track.rs:start_camera,poll_camera_job",
    ),
    (
        "camera.analyze",
        "path=analyze; resolves=comp,layer or selected-prop layers then selected layers then all,tracker effect uid/ordinal/name or selected/first; reads=editor.selection,layer.adjustment/timing,comp.frame_rate/duration,source-frame render dependencies/effects preceding tracker,tracker static settings/tracks/deleted/signatures,effect bounds,camera job; holds=tracker exists; checks=not adjustment,two frames,no running analysis; writes=analysis tracks/solve/signatures/method/error,job; source=commands/camera_cmds.rs:ct_p,analyze,camera_track.rs:start_camera,poll_camera_job",
    ),
    (
        "camera.cancel",
        "path=job; resolves=running camera job; reads=job state; holds=camera analyzing; checks=none; writes=job cancel flag; source=commands/camera_cmds.rs:specs,camera_track.rs:stop_camera",
    ),
    (
        "camera.selectPoints",
        "path=editor; resolves=active comp; reads=editor.camera_points if add/toggle; holds=comp; checks=unsigned point ids; writes=editor.camera_points; source=commands/camera_cmds.rs:select_points",
    ),
    (
        "camera.addFromSolve",
        "path=camera; resolves=tracker through ct_p,existing camera from tracker CAMERA property; reads=editor.time/selection,tracker solve/static params,tracked layer timing and transform/ancestors at in-point,comp dimensions/frame_rate/duration/layer order,existing camera.source/time mapping/property fields,next_id; holds=valid solve; checks=kind camera,existing id is camera; writes=new or existing camera transform/camera keys/values/auto_orient/remove poi,tracker CAMERA if new,next_id,selection; source=commands/camera_cmds.rs:create_from_solve,camera_track.rs:placed,write_camera",
    ),
    (
        "camera.addFromSolve",
        "path=target-layers; resolves=tracker through ct_p,points explicit or editor.camera_points,target coordinates optional,tracker CAMERA layer; reads=editor.time/selection,solve points/frames/ground,targetSize,tracked timing/transform and ancestors at in-point,comp dimensions/frame_rate/duration/layer names/order,project Solids folder,next_id; holds=valid solve; checks=kind,points form plane unless explicit target; writes=optional camera/tracker CAMERA,new text/solid/null or shadowcatcher+light layers/items,selection,next_id; source=commands/camera_cmds.rs:create_from_solve,target_p,camera_track.rs:placed,write_camera",
    ),
    (
        "camera.add",
        "path=camera; resolves=tracker through ct_p,existing camera from tracker CAMERA; reads=editor.time/selection,tracker solve/static params,tracked timing/transform/ancestors at in-point,comp dimensions/frame_rate/duration/order,existing camera.source/time mapping/fields,next_id; holds=valid solve; checks=existing id is camera; writes=new or existing camera transform/options keys/values/auto_orient/remove poi,tracker CAMERA if new,next_id,selection; source=commands/camera_cmds.rs:with_kind,create_from_solve,camera_track.rs:placed,write_camera",
    ),
    (
        "camera.setGroundPlane",
        "path=solve; resolves=tracker through ct_p,points explicit or editor.camera_points; reads=editor.time/selection,tracker solve,tracked layer transform/ancestors at in-point/timing,comp dimensions/frame_rate/duration,solve points/frames/ground; holds=solved camera,nonempty points; checks=points form plane; writes=tracker SOLVE static JSON with ground; source=commands/camera_cmds.rs:set_ground,camera_track.rs:placed",
    ),
    (
        "camera.deletePoints",
        "path=delete; resolves=tracker through ct_p,points explicit or editor.camera_points; reads=editor.selection,tracker deleted/autoDeletePoints/solve and analysis settings/tracks/signatures,source render dependencies if reanalysis,layer timing/adjustment,comp frame_rate/duration,camera job; holds=nonempty points; checks=tracker exists,reanalysis when no running camera job; writes=tracker DELETED and analysis results,editor.camera_points,job; source=commands/camera_cmds.rs:delete_points,camera_track.rs:start_camera",
    ),
    (
        "camera.move",
        "path=rig; resolves=comp,target layer from target parameter,layers list or target; reads=editor.time,comp dimensions/frame_rate/duration/layer names,layer source and text value at start or effect bounds/source size,target evaluated layer/ancestor transforms/expression dependencies unless explicit point,next_id; holds=comp; checks=preset,numeric values,ease,layer references; writes=new camera/rig layers and keys,parent,requested layers.three_d/motion_blur,comp motion_blur/shutter/samples,next_id,selection; source=commands/promo_camera.rs:camera_move,layer_center,promo_ease.rs:motion_blur",
    ),
    (
        "camera.move",
        "path=parallax; resolves=comp,layers explicit far-to-near; reads=editor.time,comp.frame_rate,layer timing/position static/keys/spatial,layer effect bounds/source refs,effects names/order,next_id; holds=at least two layers; checks=speeds count/direction/ease/finite numbers; writes=position replacement keys,depth blur/desaturate effects,layer/comp motion blur/shutter/samples,next_id; source=commands/promo_camera.rs:parallax,depth_fx,promo_ease.rs:set_keys,motion_blur",
    ),
    (
        "light.spotlight",
        "path=create; resolves=comp,target parameter,optional layers for insertion; reads=editor.time,comp dimensions/pixel_aspect/duration/frame_rate,layer names/order for target expression name-vs-index reference,project Solids folder,next_id; holds=target exists; checks=radius/numbers/ease/layer references; writes=new solid item/layer/opacity keys/effects/expressions,next_id,selection; source=commands/promo_light.rs:spotlight,layer_ref",
    ),
    (
        "light.sheen",
        "path=effects; resolves=comp,layer/selected-first; reads=editor.time,comp.frame_rate,layer source/effect bounds and source refs,layer timing,effects tree/names,next_id; holds=effects group; checks=numbers/ease; writes=effects append Progress slider keys and sheen effects/expressions,next_id; source=commands/promo_light.rs:sheen",
    ),
    (
        "layer.liftShadow",
        "path=effects; resolves=comp,layer/selected-first; reads=editor.time,comp.frame_rate,layer source/effect bounds and source refs,layer timing,effects tree/names,next_id; holds=effects group; checks=elevation/keys/to/ease; writes=effects append Elevation slider keys and shadow effects/expressions,next_id; source=commands/promo_light.rs:lift_shadow",
    ),
    (
        "transition.iris",
        "path=iris; resolves=comp,incoming layer/selection,center point or layer ref; reads=editor.time,comp dimensions/duration/frame_rate,layer timing/masks count/effect names,center/incoming names and layer stack for generated expression references,next_id; holds=incoming masks/effects groups; checks=numbers/ease/ring/center; writes=incoming Iris slider keys and mask/path expression,optional ring shape layer/expression/timing,next_id,selection; source=commands/promo_carry.rs:iris,layer_ref",
    ),
    (
        "transition.morphRect",
        "path=two-layers; resolves=comp,from/to parameters; reads=editor.time,comp.frame_rate,from start/to end evaluated content bounds/expressions/source refs/transforms,raw anchor/position/scale/rotation/opacity keys,parents/three_d,shape roundness if radius absent,stack order,masks count,target property hold_only and keys,next_id; holds=distinct unparented unrotated 2d layers with masks and visible bounds; checks=numbers/ease/radius; writes=both position/scale/opacity spliced keys and appended keyed masks,layer/comp motion blur/shutter/samples,next_id,selection; source=commands/promo_carry.rs:morph_rect,placement,local_rect,comp_rect,splice",
    ),
    (
        "transition.zoomThrough",
        "path=two-layers; resolves=comp,layer/selection,toLayer parameter; reads=editor.time,comp dimensions/frame_rate,from start/to landed raw anchor/position/scale/rotation/opacity keys,parents/three_d,evaluated local content/source bounds and transforms for default target,layer stack order,property hold_only/keys; holds=distinct unparented unrotated 2d layers; checks=size smaller than frame,numbers/eases; writes=both position/scale/opacity spliced keys,layer/comp motion blur/shutter/samples,selection; source=commands/promo_carry.rs:zoom_through,placement,local_rect,splice,splice_hold",
    ),
    (
        "layer.shake",
        "path=expression; resolves=comp,layer parameter or camera ref or first video camera; reads=editor.time when frame absent,layer.source/video/order for top camera,position/poi/rotation property presence and enabled nonempty expressions; holds=target has transform property; checks=finite numbers/seed,camera ref is camera; writes=position/poi if camera/rotation expressions wrapping existing enabled expression; source=commands/promo_moves.rs:target,shake,apply,add_offset",
    ),
    (
        "layer.handheld",
        "path=expression; resolves=comp,layer parameter or camera ref or first video camera; reads=comp.duration for default end,layer.source/video/order for top camera,position/poi/rotation property presence and enabled nonempty expressions; holds=target has transform property; checks=end after start,holds,numbers/seed,camera ref kind; writes=position/poi if camera/rotation expressions wrapping existing enabled expression; source=commands/promo_moves.rs:target,handheld,apply,add_offset",
    ),
    (
        "layer.hop",
        "path=keys; resolves=comp,layer/selection; reads=editor.time,comp.frame_rate,layer timing/raw position/anchor/scale values and keys,evaluated content local bounds/text/shape/source dependencies,position/scale keys/hold_only; holds=layer exists; checks=to point,numbers; writes=position/scale spliced keys,layer/comp motion blur/shutter/samples,selection; source=commands/promo_moves.rs:hop,promo_carry.rs:local_rect,tr3,splice",
    ),
    (
        "layer.gather",
        "path=keys; resolves=comp,explicit layers ordered around ring; reads=editor.time,comp dimensions/frame_rate,layer timing/raw position at staggered start,position keys/hold_only; holds=nonempty layers; checks=layer refs,numbers/ease; writes=position spliced keys,layer/comp motion blur/shutter/samples,selection; source=commands/promo_moves.rs:gather,promo_carry.rs:tr3,splice",
    ),
    (
        "layer.style.add",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,existing layerStyles/style groups/order/enabled/parameter types and unsupplied values/keys/expressions,comp.global_light,next_id,first style params for reply; holds=eligible layers; checks=style name,params names/coercions; writes=missing styles/blending groups,style enabled/group enabled,supplied params,next_id,selected_props; source=commands/styles.rs:add,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.dropShadow",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing dropShadow/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.innerShadow",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing innerShadow/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.outerGlow",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing outerGlow/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.innerGlow",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing innerGlow/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.bevelEmboss",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing bevelEmboss/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.satin",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing satin/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.colorOverlay",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing colorOverlay/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.gradientOverlay",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing gradientOverlay/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.stroke",
        "path=add; resolves=comp,layers/selection eligible can_have_styles; reads=layer.source,style presence/tree/params,comp.global_light,next_id; holds=eligible layers; checks=params names/coercions; writes=missing stroke/blending groups,enabled flags,supplied params,next_id,selected_props; source=commands/styles.rs:style_cmd,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.showAll",
        "path=add-all; resolves=comp,layers/selection eligible sources; reads=layer.source,existing styles/tree/order,comp.global_light,next_id; holds=eligible layers; checks=params object cannot cover multiple styles; writes=missing styles disabled/blending groups,group enabled,next_id,selected_props; source=commands/styles.rs:show_all,add_styles,project/styles.rs:add_style",
    ),
    (
        "layer.style.removeAll",
        "path=remove; resolves=comp,layers/selection eligible sources; reads=layer.source,styles group and child uids,editor.selected_props; holds=selected layers enabled; checks=missing group skipped; writes=layer.props removes layerStyles,selected_props retain remaining; source=commands/styles.rs:remove_all",
    ),
    (
        "layer.style.remove",
        "path=remove; resolves=comp,layer/selection,style match id/display name/uid/whole group or selected style; reads=editor.selected_props,styles ids/names/tree/blending identity; holds=styles exist; checks=Blending Options cannot remove; writes=style child or whole layerStyles remove,last style removes group,selected_props retain; source=commands/styles.rs:find_style,remove",
    ),
    (
        "layer.style.toggle",
        "path=toggle; resolves=comp,layer/selection,style match id/display name/uid/whole group or selected style; reads=editor.selected_props,styles ids/names/tree,enabled if value absent; holds=style exists; checks=target group exists; writes=style/group.enabled; source=commands/styles.rs:find_style,toggle",
    ),
    (
        "layer.style.globalLight",
        "path=report; resolves=comp; reads=comp.global_light static angle/altitude; holds=comp; checks=no numeric fields supplied; writes=none (journal command); source=commands/styles.rs:global_light",
    ),
    (
        "layer.style.globalLight",
        "path=set; resolves=comp; reads=editor.time,comp.frame_rate/global_light values/keys/expressions,all layer blending group/property presence; holds=comp; checks=numeric fields; writes=global_light angle/altitude value/current key and all layer blending globalLight values/keys/expressions; source=commands/styles.rs:global_light,project/styles.rs:set_global_light",
    ),
    (
        "layer.style.convertToEditable",
        "path=unsupported; resolves=none; reads=editor.selected_layers for enablement; holds=always disabled because no convertible styles; checks=returns unsupported error; writes=none; source=commands/styles.rs:no_psd_styles,convert",
    ),
    (
        "layer.align",
        "path=align; resolves=comp,layers/selection; reads=editor.time,comp dimensions for composition,layer transform existence,evaluated text/shape/source content bounds and source refs,world/ancestor transforms/expression dependencies,layer three_d/time mapping/separated position structure/position values and keys; holds=layers with bounds,selection mode requires two; checks=edge/to; writes=position or positionX/Y/Z value/current key for nonzero moves; source=commands/align.rs:align,placed,to_parent,apply_moves",
    ),
    (
        "layer.distribute",
        "path=distribute; resolves=comp,layers/selection; reads=editor.time,evaluated content bounds/source refs/world and ancestor transforms/expression dependencies,layer three_d/time mapping/separated position structure/position values and keys; holds=three layers with bounds; checks=mode; writes=position or positionX/Y/Z value/current key for nonzero moves; source=commands/align.rs:distribute,placed,to_parent,apply_moves",
    ),
    (
        "layer.nudge",
        "path=nudge; resolves=comp,layers/selection filtered unlocked; reads=editor.time,layer.locked/transform existence/three_d/time mapping/separated position structure/position values and keys,ancestor transforms/expression dependencies; holds=not all locked; checks=finite delta; writes=position or positionX/Y/Z value/current key; source=commands/align.rs:nudge,to_parent,apply_moves",
    ),
    (
        "layer.alignVideoToData",
        "path=timing; resolves=comp,layer/selection,data item id/name or first Data footage; reads=layer.source item and start_time/in_point/out_point,footage.kind/frame_rate/path/file creation date if no videoStart,host_files_only,data item.data/codec,comp.frame_rate; holds=nondata footage source,data text; checks=readable data timestamp/video timestamp; writes=layer.start_time/in_point/out_point shifted; source=commands/align_data.rs:run,data_item,data_times,parse_stamp",
    ),
    (
        "track.motion",
        "path=create; resolves=comp,layer/selection,target explicit or layer above; reads=editor.selection,layer video/source,source item size or comp dimensions,tracker count,stack order,next_id; holds=trackable pixels source; checks=target resolves; writes=new tracker group/settings/default points,next_id,current_track,selected_layers; source=commands/track.rs:create,new_with,layer_size",
    ),
    (
        "track.stabilize",
        "path=create; resolves=comp,layer/selection,target explicit else self; reads=editor.selection,layer video/source,source item size or comp dimensions,tracker count,next_id; holds=trackable pixels source; checks=target resolves; writes=new stabilize tracker/settings/default points,next_id,current_track,selected_layers; source=commands/track.rs:create,new_with,layer_size",
    ),
    (
        "track.new",
        "path=create; resolves=comp,layer/selection,target explicit/null or self for stabilize else layer above; reads=editor.selection,layer video/source,source item size or comp dimensions,tracker count,stack order,next_id; holds=trackable pixels source; checks=kind,target; writes=new tracker group/settings/default points,next_id,current_track,selected_layers; source=commands/track.rs:create,layer_size",
    ),
    (
        "track.property",
        "path=create; resolves=comp,target from layer or first selected prop,source parameter or first other trackable layer after target wrapping stack; reads=editor.selected_props,all candidate source video/kind/order,source item size or comp dimensions,source tracker count,next_id; holds=selected property enabled,trackable source; checks=source/target references; writes=source tracker group with target reference,next_id,current_track,selected_layers; source=commands/track.rs:track_property,create",
    ),
    (
        "track.select",
        "path=editor; resolves=comp,current_track layer else selected first with tracker,explicit tracker uid/name/ordinal or current/first; reads=editor.current_track/selected_layers,tracker ids/names/order; holds=tracker exists; checks=target resolution; writes=editor.current_track; source=commands/track.rs:current,select",
    ),
    (
        "track.setType",
        "path=settings; resolves=comp,layer/tracker through current_track/selection fallbacks; reads=editor.current_track/selection,tracker kind/target/settings/point count/order,layer source item size,layer above stack,next_id; holds=tracker; checks=kind; writes=tracker kind/position/rotation/scale/target,point groups add/remove,next_id; source=commands/track.rs:current,set_type",
    ),
    (
        "track.setTarget",
        "path=settings; resolves=comp,layer/tracker through current fallbacks,target parameter layer or null; reads=editor.current_track/selection,tracker.kind; holds=tracker; checks=stabilize target self or none; writes=tracker.settings.target; source=commands/track.rs:current,set_target",
    ),
    (
        "track.options",
        "path=settings; resolves=comp,layer/tracker through current fallbacks; reads=editor.current_track/selection,tracker existing options retained and returned; holds=tracker; checks=channel/action; writes=supplied tracker name/options fields; source=commands/track.rs:current,options",
    ),
    (
        "track.setPoint",
        "path=point; resolves=comp,layer/tracker through current fallbacks,nth track point; reads=editor.current_track/selection/time,comp.frame_rate,layer time mapping,track point values/keys at time,attachPointOffset,point order; holds=tracker/point; checks=point exists; writes=featureCenter and attachPoint always,supplied size/offset fields value/current key; source=commands/track.rs:current,set_point,tracking.rs:point_values",
    ),
    (
        "track.analyze",
        "path=analysis; resolves=comp,layer/tracker through current fallbacks; reads=editor.current_track/selection/time/active_comp,tracker settings/point values at first layer-time,layer timing,comp.frame_rate/duration,source frame bytes or source-render transitive dependencies,track job; holds=tracker and points; checks=direction,two frames,no running point track; writes=trackpoint featureCenter/confidence/attachPoint keys,current_track,time/job; source=commands/track.rs:analyze,analysis_times,tracking.rs:start_track,source_frame,write_frames",
    ),
    (
        "track.stop",
        "path=jobs; resolves=running point/mask/warp jobs; reads=job states; holds=some analysis running; checks=none; writes=all three job cancel flags; source=commands/track.rs:specs,tracking.rs:stop_track,mask_track.rs:stop_mask_track,warp.rs:stop_warp",
    ),
    (
        "track.apply",
        "path=transform; resolves=comp,tracker through current fallbacks,settings.target layer; reads=editor.current_track/selection,tracker settings and attachPoint keys,source timing/transforms and ancestors,first target rotation/scale,per-frame target position and parent transform,separated dimensions and target keys; holds=track results,target transform,no running point track; checks=dimensions,nonraw kind; writes=target position or separated XY,rotation/scale keys as enabled; source=commands/track.rs:apply,tracking.rs:apply,attach_track",
    ),
    (
        "track.apply",
        "path=stabilize; resolves=comp,tracker through current fallbacks,target self; reads=editor.current_track/selection,tracker settings/attachPoint keys,source timing/transforms/parent,first rotation/scale and per-frame anchor/position,separated dimensions,target keys; holds=results,target transform,no running track; checks=dimensions; writes=self anchor/position or separated XY,rotation/scale keys as enabled; source=tracking.rs:apply,attach_track",
    ),
    (
        "track.apply",
        "path=corner-pin; resolves=comp,tracker through current fallbacks,settings.target; reads=editor.current_track/selection,tracker settings/attachPoint keys,source timing/transforms/ancestors,target effect_to_comp/time mapping/effect bounds/source refs,effect stack count,next_id; holds=results,target transform/effects,no running track; checks=3 or 4 points,invertible target transform,dimensions; writes=target new Corner Pin effect/keyed corners,next_id; source=tracking.rs:apply,attach_track",
    ),
    (
        "track.apply",
        "path=raw-refusal; resolves=comp,tracker through current fallbacks; reads=tracker.kind/attachPoint keys; holds=results exist; checks=raw always errors; writes=none; source=tracking.rs:apply",
    ),
    (
        "track.reset",
        "path=reset; resolves=comp,tracker through current fallbacks; reads=editor.current_track/selection,source item size or comp dimensions,tracker kind/point count/property presence; holds=tracker; checks=none; writes=point featureCenter/featureSize/searchOffset/searchSize/confidence/attachPoint/attachPointOffset static defaults and keys clear; source=commands/track.rs:reset",
    ),
    (
        "track.delete",
        "path=delete; resolves=comp,tracker through current fallbacks; reads=editor.current_track/selection,Motion Trackers children; holds=tracker; checks=none; writes=tracker remove,empty Motion Trackers group remove,current_track clear if target; source=commands/track.rs:delete",
    ),
    (
        "track.editTargetDialog",
        "path=frontend; resolves=current tracker for enablement; reads=editor.current_track/selection,tracker existence; holds=tracker; checks=none; writes=frontend event; source=commands/track.rs:specs,has_track",
    ),
    (
        "track.optionsDialog",
        "path=frontend; resolves=current tracker for enablement; reads=editor.current_track/selection,tracker existence; holds=tracker; checks=none; writes=frontend event; source=commands/track.rs:specs,has_track",
    ),
    (
        "track.warpStabilizer",
        "path=analyze; resolves=comp,layer/selection; reads=editor.selection,layer video/adjustment/timing/effect bounds/source refs,comp.frame_rate/duration,effects preceding new warp and render dependencies,warp job,next_id; holds=video nonadjustment; checks=effect availability,two frames,no running warp; writes=new Warp Stabilizer effect,analysis/key static data,job,next_id,selected_props; source=commands/warp_cmds.rs:warp_apply,warp.rs:start_warp,poll_warp_job",
    ),
    (
        "warp.analyze",
        "path=analyze; resolves=comp,explicit layer else selected prop layers/selected layers/all,warp effect uid/ordinal/name or selected/first; reads=editor.selection,layer.adjustment/timing/effect bounds/source refs,comp.frame_rate/duration,preceding effects/render dependencies,warp detailed setting/job; holds=Warp Stabilizer; checks=two frames,not adjustment,no running warp; writes=analysis/key static data,job; source=commands/warp_cmds.rs:warp_p,warp_analyze,warp.rs:start_warp,poll_warp_job",
    ),
    (
        "warp.cancel",
        "path=job; resolves=running warp job; reads=job state; holds=warp analyzing; checks=none; writes=job cancel flag; source=commands/warp_cmds.rs:specs,warp.rs:stop_warp",
    ),
    (
        "liquify.stroke",
        "path=append; resolves=comp,layer/selection,effect explicit uid or last Liquify or create; reads=editor.time,comp.frame_rate,layer.source/timing,effect bounds/source refs,effects order/kinds,static brush size/pressure/jitter/cloneOffset defaults,distortionMesh static/keys (held previous or first),next_id; holds=AV effects layer; checks=tool/nonempty points,effect uid; writes=distortionMesh static or current key append,optional new Liquify effect,next_id; source=commands/liquify.rs:stroke,liquify_effect",
    ),
    (
        "liquify.clear",
        "path=clear; resolves=comp,layer/selection,last Liquify matching optional uid; reads=effects ids/kinds/order; holds=Liquify and distortionMesh; checks=target exists; writes=distortionMesh.value empty,keys clear; source=commands/liquify.rs:clear",
    ),
    (
        "roto.stroke",
        "path=stroke; resolves=comp,existing roto through selected/any layer fallback or layer/selection,new/first requested roto; reads=editor.roto diameter/refine diameter/selection/time,comp.frame_rate/duration,layer video/adjustment/timing/effect bounds/source refs,effect frozen/strokes/base/span and stack,next_id; holds=video nonadjustment effects layer; checks=kind/points,frame limits,not frozen,new instance requires segmentation stroke; writes=STROKES data append/base/span,optional new roto effect,next_id,selected_layers; source=commands/roto_cmds.rs:stroke,roto_p,roto.rs:frame_limits",
    ),
    (
        "roto.propagate",
        "path=propagate; resolves=comp,roto effect uid/ordinal/name or selected/first across layers; reads=editor.selection,roto strokes/base/span/frozen/evaluated params at base and expression dependencies,layer timing/source bounds,comp.frame_rate,source frames and preceding-effects render dependencies,roto job; holds=strokes exist; checks=direction,span extension not frozen,no running job unless wait; writes=optional extended STROKES span,computed cache,job; source=commands/roto_cmds.rs:propagate,roto.rs:start_roto,FrameSource",
    ),
    (
        "roto.span",
        "path=span; resolves=comp,roto through explicit or selection/first fallbacks; reads=editor.selection,roto frozen/strokes/base/span,layer timing,comp.frame_rate/duration; holds=strokes/base,not frozen; checks=frame limits and stroke min/max clamp; writes=STROKES data span; source=commands/roto_cmds.rs:span,write_data,roto.rs:frame_limits",
    ),
    (
        "roto.freeze",
        "path=freeze; resolves=comp,roto through explicit or selection/first fallbacks; reads=editor.selection,strokes/base/span/frozen/evaluated roto params at base and expression dependencies,layer timing/source bounds,comp.frame_rate,source/preceding effects render dependencies,roto job; holds=strokes; checks=no running job unless wait,strokes still same before write; writes=FROZEN data,cache/job; source=commands/roto_cmds.rs:freeze,roto.rs:start_roto,poll_roto_job,FrameSource",
    ),
    (
        "roto.unfreeze",
        "path=unfreeze; resolves=comp,roto through explicit or selection/first fallbacks; reads=editor.selection,roto.FROZEN; holds=roto exists; checks=already unfrozen returns unchanged; writes=FROZEN static empty; source=commands/roto_cmds.rs:unfreeze",
    ),
    (
        "roto.clearStrokes",
        "path=clear; resolves=comp,roto through explicit or selection/first fallbacks; reads=editor.selection,roto frozen/strokes/base/span; holds=not frozen; checks=kind enum; writes=STROKES filtered and base recalculated if removed; source=commands/roto_cmds.rs:clear_strokes,write_data",
    ),
    (
        "roto.cancel",
        "path=job; resolves=running roto job; reads=job state; holds=roto running; checks=none; writes=job cancel flag; source=commands/roto_cmds.rs:specs,roto.rs:stop_roto",
    ),
    (
        "roto.options",
        "path=editor; resolves=none; reads=editor.roto existing options; holds=always; checks=view enum,color lengths; writes=editor.roto supplied fields/clamped numbers; source=commands/roto_cmds.rs:options",
    ),
    (
        "puppet.addPin",
        "path=existing-mesh; resolves=comp,layer/selection,first Puppet,requested mesh or first under/near click; reads=editor.time/puppet defaults,comp.frame_rate,layer source/timing/effect bounds/source refs,rendered input before Puppet and transitive dependencies,evaluated Puppet mesh/pins/options/expressions/deformation,effect/mesh/pin names/order,next_id; holds=AV effects layer; checks=kind/position,requested mesh exists; writes=new pin rest/position initial animated key,selected_props,next_id; source=commands/puppet.rs:add_pin,puppet_eval",
    ),
    (
        "puppet.addPin",
        "path=new-mesh; resolves=comp,layer/selection,first Puppet or create; reads=editor.time/puppet defaults,comp.frame_rate,layer source/timing/effect bounds/source refs,existing Puppet mesh/pin names/count,next_id; holds=AV effects layer; checks=kind/position,newMesh or no matching deformed mesh; writes=optional Puppet effect,new mesh/deform/new pin rest/position initial key,selected_props,next_id; source=commands/puppet.rs:add_pin",
    ),
    (
        "puppet.movePin",
        "path=move; resolves=comp,layer/selection,pin uid/name; reads=editor.time,comp.frame_rate,layer timing,Puppet pin names/kind,position keys,rest for nonmoving region; holds=pin with position; checks=position input; writes=position value/current key,nonmoving pin rest static,selected_props; source=commands/puppet.rs:with_pin,move_pin",
    ),
    (
        "puppet.setPin",
        "path=fields; resolves=comp,layer/selection,pin uid/name; reads=editor.time,comp.frame_rate,layer timing,pin names/property availability/keys; holds=pin; checks=nonempty supplied fields,pin property exists; writes=supplied position/scale/rotation/amount/extent/in_front value/current key,selected_props; source=commands/puppet.rs:with_pin,set_pin",
    ),
    (
        "puppet.removePin",
        "path=remove; resolves=comp,pin/pins uid/name on layer or selected pins in active comp; reads=editor.selected_props,layer Puppet tree/pin parents/names; holds=pins; checks=pin refs; writes=parent.children remove pins,selected_props retain; source=commands/puppet.rs:pins_p,remove_pin",
    ),
    (
        "puppet.selectPins",
        "path=editor; resolves=comp,pin/pins uid/name or selected pins; reads=editor.selected_props/selected_keys/selected_layers,pin tree/names; holds=comp; checks=pin refs except empty array; writes=selected_props/selected_keys/selected_layers; source=commands/puppet.rs:select_pins,pins_p",
    ),
    (
        "puppet.mesh",
        "path=defaults; resolves=none if no layer/selection; reads=editor.puppet/selected_layers; holds=always; checks=no layer or no fields; writes=editor.puppet show_mesh/default density/expansion/triangles; source=commands/puppet.rs:mesh_opts",
    ),
    (
        "puppet.mesh",
        "path=mesh; resolves=comp,layer/selection,first Puppet,all meshes or mesh uid; reads=editor.time/selected_layers,comp.frame_rate,layer timing,mesh property presence/keys; holds=Puppet effect,matching mesh; checks=layer resolves,fields numeric; writes=mesh density/expansion/triangles value/current key,editor.puppet defaults/show_mesh; source=commands/puppet.rs:mesh_opts",
    ),
    (
        "puppet.recordPin",
        "path=single; resolves=comp,layer/selection,pin uid/name; reads=editor.time/puppet record speed/smoothing,comp.frame_rate/duration,layer timing,pin.kind/position keys; holds=moving pin; checks=nonempty finite samples,position exists; writes=position spliced/smoothed keys/static first sample,selected_props/keys,time; source=commands/puppet.rs:record_pin,anim_tools.rs:smooth_keys",
    ),
    (
        "puppet.recordPin",
        "path=multiple; resolves=comp,layer/selection,pin plus pins uid/name; reads=editor.time/puppet record speed/smoothing,comp.frame_rate/duration,layer timing,additional pins.position values at start,all pin kinds/keys; holds=moving pins; checks=finite samples,positions exist; writes=all pin position spliced/smoothed keys/static first sample,selection/time through recursive calls; source=commands/puppet.rs:record_pin",
    ),
    (
        "puppet.follow",
        "path=expressions; resolves=comp,layer/leader uid/name plus pins or first selected pin and other selected pins on same layer; reads=editor.selection,Puppet pin names/kinds/rest/position property uids/tree reference path,followers.position expressions for assigned field; holds=leader/followers move; checks=nonempty followers,reference constructible; writes=followers.position expressions ordered by rest distance; source=commands/puppet.rs:follow,link.rs:reference",
    ),
    (
        "puppet.recordOptions",
        "path=editor; resolves=none; reads=editor.puppet existing options; holds=always; checks=numbers clamped; writes=editor.puppet record_speed/record_smoothing/record_draft/record_show_mesh; source=commands/puppet.rs:record_options",
    ),
    (
        "paint.options",
        "path=editor; resolves=none; reads=editor.paint for partial merge and preset deselection; holds=always; checks=mode/channels/duration/erase/preset enums,numbers clamped,validation precedes reset; writes=editor.paint merged options or reset defaults; source=commands/paint.rs:options,apply_options,set_preset",
    ),
    (
        "paint.brushPreset",
        "path=editor; resolves=preset index or normalized preset name; reads=editor.paint to preserve other options; holds=always; checks=preset exists; writes=editor.paint tip fields and preset (no selector returns preset list); source=commands/paint.rs:preset,set_preset",
    ),
    (
        "paint.setCloneSource",
        "path=editor; resolves=comp,layer id/name/position or selected-first; reads=editor.selection for default layer; holds=comp; checks=point pair,layer exists; writes=editor.paint clone_source/clone_point and clone_offset clear; source=commands/paint.rs:set_clone_source",
    ),
    (
        "paint.stroke",
        "path=record; resolves=comp,layer id/name/position or selected-first; reads=editor.paint/time,comp.frame_rate,layer.start_time/stretch/source,effect bounds and source item sizes,Paint effects order/count and last effect identity,existing stroke names,project.next_id; holds=comp; checks=nonempty numeric points,kind/options enums,AV source and effects group; writes=Paint effect reuse or append,new stroke group/write-on End keys,next_id,selected_props; source=commands/paint.rs:stroke,paint_effect,paint::next_name,stroke_group",
    ),
    (
        "paint.stroke",
        "path=clone-or-eraser; resolves=paint layer,clone_source numeric ref stored in stroke; reads=editor.paint aligned/clone_point/clone_offset/clone_source and clone timing defaults,last noneraser Paint stroke uid for lastStrokeOnly,editor.paint aligned/clone_offset again when recording; holds=record path premises; checks=clone position pair when supplied; writes=clone or eraser stroke properties,editor.paint.clone_offset if aligned and absent; source=commands/paint.rs:stroke",
    ),
    (
        "paint.removeStroke",
        "path=remove; resolves=comp,layer,stroke uid or exact name across Paint effects in order; reads=effects identity/children and stroke uid/name/match_id; holds=comp; checks=matching brush/clone/eraser exists; writes=matching stroke children removed from first matching Paint effect,selected_props clear; source=commands/paint.rs:remove_stroke",
    ),
    (
        "layer.autoTrace",
        "path=same-layer; resolves=comp,layer id/name/position or selected-first; reads=editor.time,comp.frame_rate/work_area,layer.source/in_point/out_point/start_time/stretch,source_frame raw footage or rendered source and transitive render dependencies,source size,masks.children count,next_id; holds=selected layer enabled; checks=AV source,channel enum,nonempty frame range and traced contours,masks group; writes=mask groups/paths and multi-frame path/opacity Hold keys,next_id,selected_layers; source=commands/autotrace.rs:run,tracking.rs:source_frame",
    ),
    (
        "layer.autoTrace",
        "path=new-layer; resolves=comp,source layer,project Solids folder by name; reads=same-layer trace inputs,source.name/source size/timing/stretch/parent/entire transform,comp dimensions/pixel_aspect/duration/layer order,next_id and source item label; holds=selected layer enabled; checks=trace succeeds; writes=new solid item and layer above source,copied transform with fresh uids,new mask groups/keys,next_id,selected_layers; source=commands/autotrace.rs:run,project/build.rs:layer",
    ),
    (
        "contentFill.set",
        "path=editor; resolves=referenceLayer in active comp by id/name/position; reads=editor.content_fill/time/active_comp for partial merge and default reference time; holds=always; checks=method/range/lighting enums,reference layer exists,alpha expansion clamped; writes=editor.content_fill; source=commands/content_fill.rs:set",
    ),
    (
        "contentFill.generate",
        "path=plan; resolves=comp,layer/selected-first,optional referenceLayer through active comp; reads=editor.content_fill/time/selection,layer.name/source/in_point/out_point,source item size,comp.frame_rate/work_area/duration,reference layer existence,session.path and services.exists for default output directory,offload availability; holds=selected layer enabled; checks=AV source with item,pixels,range overlaps layer and comp,setting enums; writes=contentFill worker/background job or synchronous task; source=commands/content_fill.rs:generate,set,default_dir",
    ),
    (
        "contentFill.generate",
        "path=render-apply; resolves=planned comp/source and optional reference layer,re-resolves current comp/source on completion; reads=layer_input before effects with masks and transitive source/render/expression dependencies at all planned times and reference time,current source parent/three_d/entire transform,current comp.frame_rate/duration/dimensions/order,Fill folder identity/child count,first output PNG importer probe,next_id; holds=source still exists; checks=transparent hole exists,opaque reference accepted,job cancellation/file writes/importer failures; writes=PNG sequence via services,new Fill folder/footage item/layer above source,copied transform with fresh uids,next_id,selected_layers; source=commands/content_fill.rs:run_plan,masked_frame,add_fill_layer",
    ),
    (
        "layer.newContentAwareFill",
        "path=panel; resolves=none; reads=params object emptiness; holds=always; checks=empty object; writes=frontend contentAwareFill panel event; source=commands/content_fill.rs:open_panel",
    ),
    (
        "layer.newContentAwareFill",
        "path=generate; resolves=comp,layer/selected-first,referenceLayer in active comp; reads=contentFill.generate plan and render-apply premises including editor settings,source/reference render dependencies,current source transform and Fill folder; holds=nonempty params; checks=contentFill.generate checks; writes=contentFill job,PNG sequence,new item/layer and selection on completion; source=commands/content_fill.rs:open_panel,generate,run_plan,add_fill_layer",
    ),
    (
        "templateProps.setPrimary",
        "path=editor; resolves=explicit comp id/name or active comp; reads=editor.active_comp when comp omitted,project item names for named comp; holds=comp enabled; checks=comp exists; writes=editor.template_props_primary; source=commands/template_props.rs:set_primary",
    ),
    (
        "templateProps.setName",
        "path=name; resolves=explicit comp else valid template_props_primary else active comp; reads=editor.template_props_primary,comp.graphics_template presence,item.name for initial EG name,post-edit EG synchronization; holds=comp enabled; checks=name string; writes=comp.graphics_template.name,editor.template_props_primary if absent,post-edit synchronized instance groups; source=commands/template_props.rs:set_name,eg_comp,eg_of,template_props::sync_project",
    ),
    (
        "templateProps.addProperty",
        "path=controls; resolves=explicit comp/layer/property uid or path else selected_leaf_props from active comp with selected groups expanded; reads=editor.selected_props/selected_layers/template_props_primary,property.value/ui/name,type eligibility,layer.name,EG tree/driver links/existing as_type/name/group/order,next_id,post-edit EG synchronization; holds=selected properties enabled; checks=nonempty properties,supported type,as enum/eligibility,group exists; writes=new property or mirror controls or duplicates skipped,EG tree insertion,next_id,template_props_primary if absent,synchronized instance groups; source=commands/template_props.rs:add_property,prop.rs:prop_ref,template_props::control_type,can_show_as,sync_project",
    ),
    (
        "templateProps.addMirror",
        "path=mirror; resolves=eg_comp,control id or exact/case-insensitive flattened name,master through mirrors; reads=EG controls/kinds/names/group/order,next_id,post-edit EG synchronization; holds=nonempty EG enabled; checks=master is property control,group exists; writes=new mirror control,next_id,synchronized instance groups; source=commands/template_props.rs:add_mirror,control_p,template_props::resolve,sync_project",
    ),
    (
        "templateProps.linkProperty",
        "path=link; resolves=eg_comp,control id/name and master,explicit target property else first selected leaf; reads=editor.selection/template_props_primary,EG tree/links/as_type,target and master property.value/ui/type,post-edit EG synchronization; holds=nonempty EG enabled; checks=same comp,non-Font property control,matching value kind,not already driven; writes=master.links append,linked properties values/keys time-mapped and instance groups through sync; source=commands/template_props.rs:link_property,link_target,template_props::source_prop,linkable,sync_links,sync_project",
    ),
    (
        "templateProps.unlinkProperty",
        "path=unlink; resolves=eg_comp,control id/name and master,explicit target else first selected leaf; reads=editor.selection/template_props_primary,EG mirror resolution/links,post-edit EG synchronization; holds=nonempty EG enabled; checks=target is linked to resolved property control; writes=master.links removal,synchronized instance groups; source=commands/template_props.rs:unlink_property,template_props::sync_project",
    ),
    (
        "templateProps.addMedia",
        "path=media; resolves=comp,layer/selected-first,optional EG group id; reads=layer.source/name,EG flattened media controls/group/order,item.name if EG absent,next_id,editor.template_props_primary,post-edit EG synchronization; holds=comp enabled; checks=footage layer,no existing media control for layer,group exists; writes=new Media control,next_id,template_props_primary if absent,synchronized instance media properties; source=commands/template_props.rs:add_media,template_props::sync_project",
    ),
    (
        "templateProps.addGroup",
        "path=group; resolves=eg_comp; reads=editor.template_props_primary,comp.graphics_template/item.name,EG top-level order/length,next_id,post-edit EG synchronization; holds=comp enabled; checks=comp exists,index clamped; writes=new top-level Group control,next_id,template_props_primary if absent,synchronized instance groups; source=commands/template_props.rs:add_group,template_props::sync_project",
    ),
    (
        "templateProps.addComment",
        "path=comment; resolves=eg_comp,optional EG group id; reads=editor.template_props_primary,comp.graphics_template/item.name,EG group/order,next_id,post-edit EG synchronization; holds=comp enabled; checks=group exists; writes=new Comment control,next_id,template_props_primary if absent,synchronized instance groups; source=commands/template_props.rs:add_comment,template_props::sync_project",
    ),
    (
        "templateProps.rename",
        "path=rename; resolves=eg_comp,control id or exact/case-insensitive flattened name; reads=EG tree/control kind,editor.template_props_primary,post-edit EG synchronization; holds=comp enabled; checks=control exists,name or text string; writes=control.name or Comment.text,synchronized instance property/group names; source=commands/template_props.rs:rename,template_props::sync_project",
    ),
    (
        "templateProps.remove",
        "path=remove; resolves=eg_comp,control id/name; reads=EG tree/mirrors resolved through master chains,editor.template_props_primary,post-edit EG synchronization; holds=comp enabled; checks=control exists; writes=control subtree and its mirrors removed,synchronized instance groups/override lists; source=commands/template_props.rs:remove,template_props::mirrors_of,sync_project",
    ),
    (
        "templateProps.move",
        "path=move; resolves=eg_comp,control id/name,optional destination group; reads=EG control subtree/kind,source and destination ordering/length,editor.template_props_primary,post-edit EG synchronization; holds=comp enabled; checks=no self-parent,no nested groups,destination is group,index clamped; writes=control remove/reinsert,synchronized instance groups; source=commands/template_props.rs:move_control,insert,template_props::sync_project",
    ),
    (
        "templateProps.soloSupported",
        "path=editor; resolves=none; reads=editor.template_props_solo for toggle; holds=comp enabled; checks=none; writes=editor.template_props_solo; source=commands/template_props.rs:solo_supported",
    ),
    (
        "effect.editDropdown",
        "path=items; resolves=comp,layer,explicit property uid/path else first Dropdown Menu Control effect menu; reads=effects stack identity/order,property.ui/static value/all keys,post-edit EG synchronization; holds=comp enabled; checks=nonempty string items,popup property; writes=property.ui options,static/key Enum values clamped,synchronized linked/instance properties; source=commands/template_props.rs:edit_dropdown,prop.rs:prop_ref,template_props::sync_project",
    ),
    (
        "comp.openInGraphicsTemplates",
        "path=editor; resolves=comp id/name or active comp; reads=project item name for reply; holds=comp enabled; checks=comp exists; writes=editor.template_props_primary,frontend graphicsTemplates panel event; source=commands/template_props.rs:open_in_eg",
    ),
    (
        "templateProps.set",
        "path=media; resolves=comp,precomp layer/selected-first,source comp control id/name,instance property by eg control match_id,replacement footage id or project item name; reads=layer.source/templateProps group,source EG flattened names,instance property uid,replacement item.kind/has_video,post-edit EG synchronization; holds=selected precomp with Template Properties enabled; checks=precomp instance,control property exists,video footage replacement; writes=instance property scalar item id,override and mirror state through sync; source=commands/template_props.rs:set_override,instance,master_uid,template_props::sync_project",
    ),
    (
        "templateProps.set",
        "path=value; resolves=precomp instance/source control and instance property uid; reads=editor.time/time_of(comp),prefs.general.default_spatial_linear,source EG control.as_type,instance property current value/keys/ui/spatial,layer.start_time/stretch,post-edit EG synchronization; holds=selected instance enabled; checks=value coercion,Font value shape,slider range; writes=instance property value/current key via prop.set,override/mirror state through sync; source=commands/template_props.rs:set_override,font_value,prop.rs:set,with_prop,template_props::sync_project",
    ),
    (
        "templateProps.revert",
        "path=overrides; resolves=precomp instance/source comp,explicit numeric control expanded to master/mirrors else all overridden controls; reads=instance templateProps group match_ids/overridden/keys/expr,source EG mirror graph/source property metadata and values at zero/source media,post-edit EG synchronization; holds=selected instance enabled; checks=instance exists (empty targets no-op); writes=override uid removal,instance keys/expr clear,source defaults restored and mirror state synchronized; source=commands/template_props.rs:revert,targets,template_props::sync_project,build_group",
    ),
    (
        "templateProps.pushToComp",
        "path=push; resolves=precomp instance/source comp,control targets expanded to master/mirrors or all overridden,source control property/media links; reads=instance property value/keys,source EG graph/as_type,source target value/keys and layer.start_time/stretch for key mapping,instance overridden/match_ids,post-edit EG synchronization; holds=selected instance enabled; checks=instance exists,missing source controls/properties skipped; writes=source property value/keys or font fields across static/key text or source footage ref,instance overrides removed and keys/expr cleared,synchronized linked/mirror/instance properties; source=commands/template_props.rs:push_to_comp,targets,template_props::sync_project",
    ),
    (
        "templateProps.exportTemplate",
        "path=export; resolves=eg_comp,transitive source items including template media candidates; reads=editor.template_props_primary,comp dimensions/frame_rate/duration/poster_time,EG full control graph/property/linked values and metadata,all transitive item data/project.settings/next_id,footage file paths/sequences/embedded data,services.read_file,poster render and transitive render dependencies; holds=nonempty EG enabled; checks=path string,nonempty controls,file write errors (missing media/poster skipped); writes=template archive via services.write_file,toast; source=commands/template_props.rs:export_template,manifest_controls,control_json,template_props::dependencies",
    ),
    (
        "templateProps.importTemplate",
        "path=import; resolves=optional target comp id/name/active comp unless addToComp false,template comp from archive; reads=services.read_file/template archive and media entries,project.next_id,existing all item/layer/property/control IDs for fix_next_id,optional target dimensions/frame_rate/duration/renderer/order and selected_layers for layer insertion,post-edit EG synchronization; holds=always; checks=template format/version/project data,optional target containment when adding instance; writes=extracted media through services,new folder and offset imported items/IDs,project_selection,next_id,optional target precomp layer and selected_layers/selected_props/selected_keys,synchronized instance groups,toast; source=commands/template_props.rs:import_template,read_template,template_props::offset_ids,layer.rs:add_item,insert_layer_at,template_props::sync_project",
    ),
    (
        "templateProps.setName",
        "path=post-edit-sync; resolves=all comps and precomp source comps,all EG controls/master/mirror/link references; reads=before/after all EG control graphs,linked property types/value/keys and layer timing,instance source/match_ids/metadata/values/keys/expr/overridden,source property values at zero/source media,next_id; holds=any s.edit invokes templateProps sync including commands outside this module; checks=missing controls/properties skipped,mirror depth bounded; writes=linked property value/time-mapped keys,instance override/mirror groups/values/keys/expr,group insertion/removal,next_id; source=project/template_props.rs:sync_project,sync_links,propagate_mirrors,mark_overrides,build_group",
    ),
];

// Frozen from the source rows after declaration exposure. p/s/t mean part/selection/time.
// Roles include the target and secondary selectors; property, control and point uids are not objects.
pub(crate) const DECLARATIONS: &[(&str, &str, &str, &str)] = &[
    ("mask.new", "ps", "layer", "comp:Item,layer:Layer"),
    ("mask.addVertex", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.setVertex", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.setClosed", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.selectVertices", "ps", "", "vertices[].layer:Layer"),
    ("mask.moveVertices", "pst", "", "vertices[].layer:Layer"),
    ("mask.deleteVertices", "pst", "", "vertices[].layer:Layer"),
    ("mask.convertVertex", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.insertVertex", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.featherPoint.add", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.featherPoint.set", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.featherPoint.remove", "pst", "layer", "comp:Item,layer:Layer"),
    ("mask.remove", "ps", "layer", "comp:Item,layer:Layer"),
    ("mask.removeAll", "ps", "layer", "comp:Item,layer:Layer"),
    ("path.rotoBezier", "ps", "layer", "comp:Item,layer:Layer,layers:Layer"),
    ("path.convertToBezier", "pst", "layer", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.mask.motionBlur", "ps", "layer", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.mask.featherFalloff", "ps", "layer", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.mask.hideLocked", "p", "", ""),
    ("path.setFirstVertex", "ps", "layer", "comp:Item,layer:Layer"),
    ("path.freeTransform", "pst", "layer", "comp:Item,layer:Layer,layers:Layer"),
    ("path.groupShapes", "ps", "layer", "comp:Item,layer:Layer"),
    ("path.ungroupShapes", "ps", "layer", "comp:Item,layer:Layer"),
    ("mask.interpolate", "ps", "layer", "comp:Item,layer:Layer"),
    ("mask.interpolationOptions", "", "", ""),
    ("track.maskMethod", "", "", ""),
    ("track.mask", "pst", "layer", "comp:Item,layer:Layer"),
    ("track.extractFaceMeasurements", "pst", "layer", "comp:Item,layer:Layer"),
    ("shape.dashes.add", "ps", "layer", "comp:Item,layer:Layer"),
    ("shape.dashes.remove", "ps", "layer", "comp:Item,layer:Layer"),
    ("shape.stroke.taper", "pst", "layer", "comp:Item,layer:Layer"),
    ("shape.stroke.wave", "pst", "layer", "comp:Item,layer:Layer"),
    ("shape.fillStroke", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("shape.newShape", "pst", "layer", "comp:Item,layer:Layer"),
    ("shape.toolOptions", "", "", ""),
    ("path.pointsFollowNulls", "pst", "layer", "comp:Item,layer:Layer"),
    ("path.nullsFollowPoints", "pst", "layer", "comp:Item,layer:Layer"),
    ("path.tracePath", "pst", "layer", "comp:Item,layer:Layer"),
    ("layer.newCamera", "pst", "", "comp:Item"),
    ("layer.cameraSettings", "pst", "layer", "comp:Item,layer:Layer"),
    ("layer.newLight", "pst", "", "comp:Item"),
    ("layer.lightSettings", "pst", "layer", "comp:Item,layer:Layer"),
    ("layer.autoOrient", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("view.set3DView", "p", "", "comp:Item"),
    ("view.threeD.activeCamera", "p", "", "comp:Item"),
    ("view.threeD.default", "p", "", "comp:Item"),
    ("view.threeD.front", "p", "", "comp:Item"),
    ("view.threeD.left", "p", "", "comp:Item"),
    ("view.threeD.top", "p", "", "comp:Item"),
    ("view.threeD.back", "p", "", "comp:Item"),
    ("view.threeD.right", "p", "", "comp:Item"),
    ("view.threeD.bottom", "p", "", "comp:Item"),
    ("view.threeD.custom1", "p", "", "comp:Item"),
    ("view.threeD.custom2", "p", "", "comp:Item"),
    ("view.threeD.custom3", "p", "", "comp:Item"),
    ("view.threeD.last", "p", "", "comp:Item"),
    ("view.reset3DView", "pt", "", "comp:Item"),
    ("view.set3DViewCamera", "p", "", "comp:Item"),
    ("camera.orbit", "pt", "layer", "comp:Item,layer:Layer"),
    ("camera.pan", "pt", "layer", "comp:Item,layer:Layer"),
    ("camera.dolly", "pt", "layer", "comp:Item,layer:Layer"),
    ("view.lookAtSelected", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("view.lookAtAll", "pt", "", "comp:Item"),
    ("camera.fromView", "pst", "", "comp:Item"),
    ("layer.newModel", "ps", "", "comp:Item,item:Item"),
    ("layer.new3dPrimitive", "pst", "", "comp:Item"),
    ("layer.newPrimitive", "pst", "", "comp:Item"),
    ("comp.renderer", "p", "", "comp:Item"),
    ("layer.environment", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("material.set", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("material.reset", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("material.duplicateAssign", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("material.revealSource", "ps", "layer", "comp:Item,layer:Layer"),
    ("camera.stereoRig", "pst", "", "comp:Item"),
    ("camera.orbitNull", "pst", "layer", "camera:Layer,comp:Item,layer:Layer"),
    ("light.controlWithCamera", "pst", "layers", "camera:Layer,comp:Item,layer:Layer,layers:Layer"),
    ("camera.fromModel", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("light.fromModel", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("light.environmentBackground", "pst", "layer", "comp:Item,layer:Layer"),
    ("track.camera", "pst", "layer", "comp:Item,layer:Layer"),
    ("camera.analyze", "pst", "layer", "comp:Item,layer:Layer"),
    ("camera.cancel", "", "", ""),
    ("camera.selectPoints", "p", "", ""),
    ("camera.addFromSolve", "pst", "layer", "comp:Item,layer:Layer"),
    ("camera.add", "pst", "layer", "comp:Item,layer:Layer"),
    ("camera.setGroundPlane", "pst", "layer", "comp:Item,layer:Layer"),
    ("camera.deletePoints", "pst", "layer", "comp:Item,layer:Layer"),
    ("camera.move", "pst", "layers", "comp:Item,layers:Layer,target:Layer"),
    ("light.spotlight", "pst", "layers", "comp:Item,layers:Layer,target:Layer"),
    ("light.sheen", "pst", "layer", "comp:Item,layer:Layer"),
    ("layer.liftShadow", "pst", "layer", "comp:Item,layer:Layer"),
    ("transition.iris", "pst", "layer", "center:Layer,comp:Item,layer:Layer"),
    ("transition.morphRect", "pt", "", "comp:Item,from:Layer,to:Layer"),
    ("transition.zoomThrough", "pst", "layer", "comp:Item,layer:Layer,toLayer:Layer"),
    ("layer.shake", "pt", "layer", "camera:Layer,comp:Item,layer:Layer"),
    ("layer.handheld", "p", "layer", "camera:Layer,comp:Item,layer:Layer"),
    ("layer.hop", "pst", "layer", "comp:Item,layer:Layer"),
    ("layer.gather", "pt", "layers", "comp:Item,layers:Layer"),
    ("layer.style.add", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.dropShadow", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.innerShadow", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.outerGlow", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.innerGlow", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.bevelEmboss", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.satin", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.colorOverlay", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.gradientOverlay", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.stroke", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.showAll", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.removeAll", "ps", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.style.remove", "ps", "layer", "comp:Item,layer:Layer"),
    ("layer.style.toggle", "ps", "layer", "comp:Item,layer:Layer"),
    ("layer.style.globalLight", "pt", "", "comp:Item"),
    ("layer.style.convertToEditable", "ps", "", ""),
    ("layer.align", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.distribute", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.nudge", "pst", "layers", "comp:Item,layer:Layer,layers:Layer"),
    ("layer.alignVideoToData", "ps", "layer", "comp:Item,data:Item,layer:Layer"),
    ("track.motion", "ps", "layer", "comp:Item,layer:Layer,target:Layer"),
    ("track.stabilize", "ps", "layer", "comp:Item,layer:Layer,target:Layer"),
    ("track.new", "ps", "layer", "comp:Item,layer:Layer,target:Layer"),
    ("track.property", "ps", "layer", "comp:Item,layer:Layer,source:Layer"),
    ("track.select", "ps", "layer", "comp:Item,layer:Layer"),
    ("track.setType", "ps", "layer", "comp:Item,layer:Layer"),
    ("track.setTarget", "ps", "layer", "comp:Item,layer:Layer,target:Layer"),
    ("track.options", "ps", "layer", "comp:Item,layer:Layer"),
    ("track.setPoint", "pst", "layer", "comp:Item,layer:Layer"),
    ("track.analyze", "pst", "layer", "comp:Item,layer:Layer"),
    ("track.stop", "", "", ""),
    ("track.apply", "ps", "layer", "comp:Item,layer:Layer"),
    ("track.reset", "ps", "layer", "comp:Item,layer:Layer"),
    ("track.delete", "ps", "layer", "comp:Item,layer:Layer"),
    ("track.editTargetDialog", "ps", "", ""),
    ("track.optionsDialog", "ps", "", ""),
    ("track.warpStabilizer", "pst", "layer", "comp:Item,layer:Layer"),
    ("warp.analyze", "pst", "layer", "comp:Item,layer:Layer"),
    ("warp.cancel", "", "", ""),
    ("liquify.stroke", "pst", "layer", "comp:Item,layer:Layer"),
    ("liquify.clear", "ps", "layer", "comp:Item,layer:Layer"),
    ("roto.stroke", "pst", "layer", "comp:Item,layer:Layer"),
    ("roto.propagate", "pst", "layer", "comp:Item,layer:Layer"),
    ("roto.span", "ps", "layer", "comp:Item,layer:Layer"),
    ("roto.freeze", "pst", "layer", "comp:Item,layer:Layer"),
    ("roto.unfreeze", "ps", "layer", "comp:Item,layer:Layer"),
    ("roto.clearStrokes", "ps", "layer", "comp:Item,layer:Layer"),
    ("roto.cancel", "", "", ""),
    ("roto.options", "", "", ""),
    ("puppet.addPin", "pst", "layer", "comp:Item,layer:Layer"),
    ("puppet.movePin", "pst", "layer", "comp:Item,layer:Layer"),
    ("puppet.setPin", "pst", "layer", "comp:Item,layer:Layer"),
    ("puppet.removePin", "ps", "layer", "comp:Item,layer:Layer"),
    ("puppet.selectPins", "ps", "layer", "comp:Item,layer:Layer"),
    ("puppet.mesh", "pst", "layer", "comp:Item,layer:Layer"),
    ("puppet.recordPin", "pst", "layer", "comp:Item,layer:Layer"),
    ("puppet.follow", "pst", "layer", "comp:Item,layer:Layer"),
    ("puppet.recordOptions", "", "", ""),
    ("paint.options", "", "", "cloneSource:Layer"),
    ("paint.brushPreset", "", "", ""),
    ("paint.setCloneSource", "ps", "layer", "comp:Item,layer:Layer"),
    ("paint.stroke", "pst", "layer", "cloneSource:Layer,comp:Item,layer:Layer"),
    ("paint.removeStroke", "ps", "layer", "comp:Item,layer:Layer"),
    ("layer.autoTrace", "pst", "layer", "comp:Item,layer:Layer"),
    ("contentFill.set", "pt", "", "referenceLayer:Layer"),
    ("contentFill.generate", "pst", "layer", "comp:Item,layer:Layer,referenceLayer:Layer"),
    ("layer.newContentAwareFill", "pst", "layer", "comp:Item,layer:Layer,referenceLayer:Layer"),
    ("templateProps.setPrimary", "p", "", "comp:Item"),
    ("templateProps.setName", "p", "", "comp:Item"),
    ("templateProps.addProperty", "ps", "layer", "comp:Item,layer:Layer"),
    ("templateProps.addMirror", "p", "", "comp:Item"),
    ("templateProps.linkProperty", "ps", "layer", "comp:Item,layer:Layer"),
    ("templateProps.unlinkProperty", "ps", "layer", "comp:Item,layer:Layer"),
    ("templateProps.addMedia", "ps", "layer", "comp:Item,layer:Layer"),
    ("templateProps.addGroup", "p", "", "comp:Item"),
    ("templateProps.addComment", "p", "", "comp:Item"),
    ("templateProps.rename", "p", "", "comp:Item"),
    ("templateProps.remove", "p", "", "comp:Item"),
    ("templateProps.move", "p", "", "comp:Item"),
    ("templateProps.soloSupported", "p", "", ""),
    ("effect.editDropdown", "ps", "layer", "comp:Item,layer:Layer"),
    ("comp.openInGraphicsTemplates", "p", "", "comp:Item"),
    ("templateProps.set", "pst", "layer", "comp:Item,item:Item,layer:Layer"),
    ("templateProps.revert", "ps", "layer", "comp:Item,layer:Layer"),
    ("templateProps.pushToComp", "ps", "layer", "comp:Item,layer:Layer"),
    ("templateProps.exportTemplate", "p", "", "comp:Item"),
    ("templateProps.importTemplate", "pst", "", "comp:Item"),
];

// Active selectors and editor modes have no CallContext setter. No row reads a pixel Area.
// The proof is each row's named state field, not the production PST-only read table.
pub(crate) const EXTENDED_EDITOR: &[(&str, &str)] = &[
    ("layer.mask.hideLocked", "mode"),
    ("mask.interpolate", "mode"),
    ("mask.interpolationOptions", "mode"),
    ("track.mask", "mode"),
    ("shape.newShape", "mode"),
    ("shape.toolOptions", "mode"),
    ("view.set3DView", "mode"),
    ("view.threeD.activeCamera", "mode"),
    ("view.threeD.default", "mode"),
    ("view.threeD.front", "mode"),
    ("view.threeD.left", "mode"),
    ("view.threeD.top", "mode"),
    ("view.threeD.back", "mode"),
    ("view.threeD.right", "mode"),
    ("view.threeD.bottom", "mode"),
    ("view.threeD.custom1", "mode"),
    ("view.threeD.custom2", "mode"),
    ("view.threeD.custom3", "mode"),
    ("view.threeD.last", "mode"),
    ("view.reset3DView", "mode"),
    ("view.set3DViewCamera", "mode"),
    ("camera.orbit", "mode"),
    ("camera.pan", "mode"),
    ("camera.dolly", "mode"),
    ("view.lookAtSelected", "mode"),
    ("view.lookAtAll", "mode"),
    ("camera.fromView", "mode"),
    ("track.select", "active"),
    ("track.setType", "active"),
    ("track.setTarget", "active"),
    ("track.options", "active"),
    ("track.setPoint", "active"),
    ("track.analyze", "active"),
    ("track.apply", "active"),
    ("track.reset", "active"),
    ("track.delete", "active"),
    ("track.editTargetDialog", "active"),
    ("track.optionsDialog", "active"),
    ("roto.stroke", "mode"),
    ("roto.options", "mode"),
    ("puppet.addPin", "mode"),
    ("puppet.mesh", "mode"),
    ("puppet.recordPin", "mode"),
    ("puppet.recordOptions", "mode"),
    ("paint.options", "active,mode"),
    ("paint.brushPreset", "active,mode"),
    ("paint.stroke", "active,mode"),
    ("contentFill.set", "active,mode"),
    ("contentFill.generate", "active,mode"),
    ("layer.newContentAwareFill", "active,mode"),
    ("templateProps.setName", "active"),
    ("templateProps.addProperty", "active"),
    ("templateProps.addMirror", "active"),
    ("templateProps.linkProperty", "active"),
    ("templateProps.unlinkProperty", "active"),
    ("templateProps.addMedia", "active"),
    ("templateProps.addGroup", "active"),
    ("templateProps.addComment", "active"),
    ("templateProps.rename", "active"),
    ("templateProps.remove", "active"),
    ("templateProps.move", "active"),
    ("templateProps.soloSupported", "mode"),
    ("templateProps.exportTemplate", "active"),
];

#[test]
fn independent_geometry_rows_name_registered_commands_and_unique_paths() {
    let mut paths = std::collections::BTreeSet::new();
    for &(id, row) in ROWS {
        assert!(kition_motion_engine::command_specs().iter().any(|spec| spec.id == id), "unknown command {id}");
        let fields: Vec<_> = row.split("; ").collect();
        assert_eq!(fields.len(), 7, "seven structured fields required: {id}");
        for (field, prefix) in fields.iter().zip(["path=", "resolves=", "reads=", "holds=", "checks=", "writes=", "source="]) {
            assert!(field.starts_with(prefix), "missing {prefix}: {id}");
            assert!(field.len() > prefix.len(), "empty {prefix}: {id}");
        }
        assert!(paths.insert((id, fields[0])), "duplicate command path: {id} {}", fields[0]);
    }
}

#[test]
fn geometry_declarations_match_source_roles() {
    use std::collections::{BTreeMap, BTreeSet};

    use kition_kit::KitDocument;
    use kition_motion_kit::MotionDocument;

    let doc = MotionDocument::create("Geometry declarations.kmotion", 320, 180).unwrap();
    let commands = doc.commands();
    let written: BTreeSet<_> = commands.iter().filter(|c| c.mutates).map(|c| c.id.as_str()).collect();
    let source_ids: BTreeSet<_> = ROWS.iter().map(|r| r.0).filter(|id| written.contains(id)).collect();
    let all_frozen_ids: BTreeSet<_> = DECLARATIONS.iter().map(|r| r.0).collect();
    let frozen_ids: BTreeSet<_> = all_frozen_ids.iter().copied().filter(|id| written.contains(id)).collect();
    assert_eq!(source_ids, frozen_ids, "every source write needs one frozen comparison");
    assert_eq!(all_frozen_ids.len(), DECLARATIONS.len(), "duplicate frozen command");
    let mut differences: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut differing_commands = BTreeSet::new();
    let mut blocked_roles = BTreeMap::new();
    for &(id, editor, target, roles) in DECLARATIONS {
        let actual = commands.iter().find(|c| c.id == id).unwrap();
        if !actual.mutates {
            continue;
        }
        let mut expected_editor: BTreeSet<_> = editor
            .chars()
            .map(|c| match c {
                'p' => "part",
                's' => "selection",
                't' => "time",
                _ => panic!("bad editor code"),
            })
            .map(str::to_owned)
            .collect();
        if let Some((_, extra)) = EXTENDED_EDITOR.iter().find(|row| row.0 == id) {
            expected_editor.extend(extra.split(',').map(str::to_owned));
        }
        let expected_target: BTreeSet<_> = (!target.is_empty()).then(|| target.to_owned()).into_iter().collect();
        let mut expected_roles = BTreeSet::new();
        for role in roles.split(',').filter(|role| !role.is_empty()) {
            let (name, _) = role.split_once(':').unwrap();
            let root = name.split('.').next().unwrap().trim_end_matches("[]");
            if actual.schema.as_ref().and_then(|s| s.get(root)).is_some() {
                expected_roles.insert(role.to_owned());
            } else {
                blocked_roles.entry(id).or_insert_with(Vec::new).push(role);
            }
        }
        let expected_objects: BTreeSet<_> =
            expected_roles.iter().filter_map(|role| role.split_once(':')).map(|(name, _)| name).filter(|name| *name != target).map(str::to_owned).collect();
        let actual_roles: BTreeSet<_> = actual
            .object_types
            .iter()
            .flat_map(|(name, types)| types.iter().map(move |ty| (*name, *ty)))
            .filter(|(name, ty)| {
                let Some(parent) = name.strip_suffix("[]") else { return true };
                let same_parent = actual.object_types.iter().any(|(name, types)| *name == parent && types.contains(ty));
                let scalar_array = actual.schema.as_ref().and_then(|schema| schema.get(parent)).is_some_and(|schema| {
                    let scalar = |item: &serde_json::Value| matches!(item.get("type").and_then(serde_json::Value::as_str), Some("integer" | "string"));
                    schema.get("type").and_then(serde_json::Value::as_str) == Some("array")
                        && schema.get("items").is_some_and(|items| {
                            scalar(items)
                                || items
                                    .get("oneOf")
                                    .and_then(serde_json::Value::as_array)
                                    .is_some_and(|variants| !variants.is_empty() && variants.iter().all(scalar))
                        })
                });
                !(same_parent && scalar_array)
            })
            .map(|(name, ty)| format!("{name}:{ty}"))
            .collect();
        for (class, expected, actual) in [
            ("editor", expected_editor, actual.reads.iter().map(|state| state.as_str().to_owned()).collect()),
            ("target", expected_target, actual.targets.into_iter().map(str::to_owned).collect()),
            ("objects", expected_objects, actual.objects.iter().map(|s| (*s).to_owned()).collect()),
            ("object_types", expected_roles, actual_roles),
        ] {
            let less: Vec<_> = expected.difference(&actual).cloned().collect();
            let more: Vec<_> = actual.difference(&expected).cloned().collect();
            if !less.is_empty() || !more.is_empty() {
                differing_commands.insert(id);
                differences.entry(class).or_default().push(format!("{id}: less={less:?}, more={more:?}"));
            }
        }
    }
    println!("GEOMETRY_SCHEMA_BLOCKED {} commands; {} roles", blocked_roles.len(), blocked_roles.values().map(Vec::len).sum::<usize>());
    for (id, roles) in &blocked_roles {
        println!("schema-blocked {id}: {roles:?}");
    }
    for (class, entries) in &differences {
        println!("GEOMETRY_DIFFERENCES {class}: {} commands", entries.len());
        for entry in entries {
            println!("{class} {entry}");
        }
    }
    println!("GEOMETRY_COMPARISON {} write IDs; {} differing commands", frozen_ids.len(), differing_commands.len());
    let exact_field_branches = BTreeSet::from(["path.convertToBezier", "layer.mask.motionBlur", "layer.mask.featherFalloff", "mask.setClosed"]);
    let required_indirect_branches = BTreeSet::from(["track.apply", "light.controlWithCamera", "layer.autoTrace"]);
    let runtime_checked: BTreeSet<_> = exact_field_branches.union(&required_indirect_branches).copied().collect();
    let unresolved: Vec<_> = source_ids.difference(&runtime_checked).copied().collect();
    println!(
        "GEOMETRY_FIELD_COVERAGE exact_branch_commands={} required_indirect_branch_commands={} no_runtime_field_check={}",
        exact_field_branches.len(),
        required_indirect_branches.len(),
        unresolved.len()
    );
    println!("GEOMETRY_FIELD_UNRESOLVED {unresolved:?}");
    assert!(differences.is_empty(), "source-derived geometry declaration differences in {} commands (printed above)", differing_commands.len());
}

fn geometry_mask_fixture() -> (kition_motion_kit::MotionDocument, u64, u64, u64) {
    use kition_kit::KitDocument;
    use serde_json::json;

    let mut doc = kition_motion_kit::MotionDocument::create("Geometry premises.kmotion", 320, 180).unwrap();
    let layer = doc.execute("layer.newSolid", json!({"name":"Source","color":"#336699"})).unwrap()["layer"].as_u64().unwrap();
    let mask = doc.execute("mask.new", json!({"layer":layer,"vertices":[[0,0],[20,0],[20,20]],"closed":false})).unwrap()["mask"].as_u64().unwrap();
    let comp = doc.inspect().unwrap()["comp"]["id"].as_u64().unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&doc.save(None).unwrap()).unwrap();
    let masks = &saved["items"][comp.to_string()]["kind"]["layers"][0]["props"]["children"][0];
    assert_eq!(masks["match"], "masks", "the fixture's raw KDM prefix is props.children.0");
    assert_eq!(masks["children"][0]["uid"], mask);
    doc.execute("layer.select", json!({"layers":[]})).unwrap();
    (doc, comp, layer, mask)
}

fn geometry_runtime_under(doc: &mut kition_motion_kit::MotionDocument, comp: u64, command: &str, params: serde_json::Value) -> kition_kit::Under {
    use kition_kit::{CallContext, KitDocument};
    use serde_json::json;

    doc.set_context(&CallContext { part: Some(json!({"comp":comp})), time: Some(0.0), on: Vec::new() }).unwrap();
    doc.under(command, &params).unwrap().unwrap_or_default()
}

#[test]
fn geometry_mask_scalar_fields_match_handler_assignments() {
    use kition_kit::{Premise, Under};
    use serde_json::json;

    let (mut doc, comp, layer, mask) = geometry_mask_fixture();
    let group = format!("props.children.0.children[uid={mask}]");
    let mut differences = Vec::new();
    for (command, params, assigned) in [
        ("path.convertToBezier", json!({"layer":layer,"mask":mask}), "roto_bezier"),
        ("layer.mask.motionBlur", json!({"layer":layer,"mask":mask,"mode":"off"}), "motion_blur"),
        ("layer.mask.featherFalloff", json!({"layer":layer,"mask":mask,"mode":"linear"}), "feather_falloff"),
    ] {
        // paths::masks_for probes match="path" by uid before layer_menu::target_masks;
        // edit_mask_groups then matches GroupKind::Mask and assigns exactly one scalar.
        let fields = vec![
            "props.children.0.meta.match".to_owned(),
            format!("{group}.uid"),
            format!("{group}.meta.match"),
            format!("{group}.meta.kind.kind"),
            format!("{group}.meta.kind.{assigned}"),
        ];
        let expected = Under { premises: vec![Premise::on(json!(layer)).fields(fields).positions(Vec::<String>::new())], ..Default::default() };
        let actual = geometry_runtime_under(&mut doc, comp, command, params);
        if actual != expected {
            println!(
                "GEOMETRY_FIELD_DIFFERENCE {command}: expected={}, actual={}",
                serde_json::to_string(&expected).unwrap(),
                serde_json::to_string(&actual).unwrap()
            );
            differences.push(command);
        }
    }
    assert!(
        differences.is_empty(),
        "mask scalar field premises differ: {differences:?}; an absent premise leaves the explicit layer at the SDK's whole-object default"
    );
}

#[test]
fn geometry_mask_closed_fields_match_static_path_branch() {
    use kition_kit::{Premise, Under};
    use serde_json::json;

    let (mut doc, comp, layer, mask) = geometry_mask_fixture();
    let group = format!("props.children.0.children[uid={mask}]");
    // mask::path_group resolves the uid; edit_path_counted reads layer_time, kind and
    // the static path plus key emptiness. setClosed neither evaluates expressions nor
    // reads another property or the layer's place in the composition's stack.
    let fields = vec![
        "attrs.stretch".to_owned(),
        "props.children.0.meta.match".to_owned(),
        format!("{group}.uid"),
        format!("{group}.meta.kind.kind"),
        format!("{group}.meta.kind.roto_bezier"),
        format!("{group}.children.0.meta.match"),
        format!("{group}.children.0.value"),
        format!("{group}.children.0.keys"),
    ];
    let expected = Under { premises: vec![Premise::on(json!(layer)).fields(fields).positions(["startTime"])], ..Default::default() };
    let actual = geometry_runtime_under(&mut doc, comp, "mask.setClosed", json!({"layer":layer,"mask":mask,"closed":true}));
    println!(
        "GEOMETRY_FIELD_DIFFERENCE mask.setClosed: expected={}, actual={}",
        serde_json::to_string(&expected).unwrap(),
        serde_json::to_string(&actual).unwrap()
    );
    assert_eq!(actual, expected, "static mask.setClosed needs its path and timing, not every field and position of the layer");
}

// These checks freeze required indirect dependencies, not an exhaustive read set for
// expression evaluation, property-tree membership, rendering or Template Properties sync.
fn geometry_required_premises(command: &str, actual: &kition_kit::Under, expected: &[kition_kit::Premise]) {
    use std::collections::BTreeSet;

    let mut differences = Vec::new();
    for expected in expected {
        let found = actual.premises.iter().find(|premise| premise.id == expected.id);
        for (class, wanted, got) in [
            ("fields", expected.fields.as_ref(), found.map(|premise| premise.fields.as_ref())),
            ("positions", expected.positions.as_ref(), found.map(|premise| premise.positions.as_ref())),
        ] {
            let wanted = wanted.unwrap();
            let less = match got {
                Some(None) => Vec::new(),
                Some(Some(got)) => {
                    let got: BTreeSet<_> = got.iter().collect();
                    wanted.iter().filter(|field| !got.contains(field)).cloned().collect()
                }
                None => wanted.clone(),
            };
            if !less.is_empty() {
                differences.push(format!("{}.{} less={less:?}", expected.id, class));
            }
        }
    }
    println!("GEOMETRY_REQUIRED_PREMISES {command}: {}", differences.join("; "));
    assert!(differences.is_empty(), "required source-derived indirect premises missing for {command}: {differences:?}");
}

fn geometry_null_matrix_premise(id: u64) -> kition_kit::Premise {
    kition_kit::Premise::on(serde_json::json!(id))
        .fields([
            "attrs.source",
            "attrs.stretch",
            "switches.three_d",
            "attrs.auto_orient",
            "parent",
            "transform/positionX",
            "transform/position",
            "transform/rotation",
            "transform/anchor",
            "transform/scale",
        ])
        .positions(["startTime"])
}

#[test]
fn geometry_track_apply_requires_stored_target_and_parent_chain() {
    use kition_kit::{KitDocument, Premise};
    use kition_motion_engine::Session;
    use kition_motion_project::{ItemId, LayerId};
    use serde_json::json;

    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Track","width":32,"height":32,"duration":2})).unwrap()["comp"].as_u64().unwrap();
    let source = session.execute("layer.newSolid", json!({"name":"Source"})).unwrap()["layer"].as_u64().unwrap();
    let target = session.execute("layer.newNull", json!({"name":"Target"})).unwrap()["layer"].as_u64().unwrap();
    let parent = session.execute("layer.newNull", json!({"name":"Parent"})).unwrap()["layer"].as_u64().unwrap();
    let ancestor = session.execute("layer.newNull", json!({"name":"Ancestor"})).unwrap()["layer"].as_u64().unwrap();
    session.execute("layer.setParent", json!({"layers":[target],"parent":parent,"compensate":false})).unwrap();
    session.execute("layer.setParent", json!({"layers":[parent],"parent":ancestor,"compensate":false})).unwrap();
    let tracker = session
        .execute("track.new", json!({"layer":source,"target":target,"kind":"transform","position":true,"rotation":false,"scale":false}))
        .unwrap()["tracker"]
        .as_u64()
        .unwrap();
    let source_layer = session.project.comp(ItemId(comp)).unwrap().layer(LayerId(source)).unwrap();
    let solid = source_layer.source.item().unwrap().0;
    let attach = source_layer.tracker(tracker).unwrap().0.track_points().next().unwrap().get("attachPoint").unwrap().uid;
    for (time, value) in [(0, [8, 8]), (1, [10, 10])] {
        session.execute("prop.addKey", json!({"layer":source,"prop":attach,"time":time,"value":value})).unwrap();
    }
    let mut doc = kition_motion_kit::MotionDocument::open("Track premises.kmotion", session.project.to_json().as_bytes()).unwrap();
    let params = json!({"layer":source,"tracker":tracker,"dimensions":"xy"});
    let actual = geometry_runtime_under(&mut doc, comp, "track.apply", params.clone());
    assert_eq!(doc.execute("track.apply", params).unwrap()["frames"], 2, "this is the Transform apply branch with a real analyzed track");
    let expected = vec![
        Premise::on(json!(target))
            .fields(["attrs.source", "attrs.stretch", "parent", "transform/rotation", "transform/scale", "transform/position", "transform/positionX"])
            .positions(["startTime"]),
        geometry_null_matrix_premise(parent),
        geometry_null_matrix_premise(ancestor),
        Premise::on(json!(solid)).fields(["kind", "data.pixel_aspect"]).positions(Vec::<String>::new()),
        Premise::on(json!(comp)).fields(["comp.attrs.pixel_aspect"]).positions(Vec::<String>::new()),
    ];
    // tracking::apply reads settings.target, target timing/initial transform, and
    // comp_to_parent -> EvalCtx::layer_to_comp/world_matrix for both null ancestors.
    // The source's pixel aspect comes from its solid Item, outside the source layer.
    geometry_required_premises("track.apply", &actual, &expected);
}

#[test]
fn geometry_release_light_requires_camera_parent_matrix() {
    use kition_kit::KitDocument;
    use kition_motion_engine::Session;
    use serde_json::json;

    let mut session = Session::default();
    let comp = session.execute("comp.new", json!({"name":"Rig","width":32,"height":32,"duration":2})).unwrap()["comp"].as_u64().unwrap();
    let camera = session.execute("layer.newCamera", json!({"name":"Camera"})).unwrap()["layer"].as_u64().unwrap();
    let light = session.execute("layer.newLight", json!({"name":"Light","kind":"spot"})).unwrap()["layer"].as_u64().unwrap();
    let parent = session.execute("layer.newNull", json!({"name":"Rig parent"})).unwrap()["layer"].as_u64().unwrap();
    session.execute("layer.setParent", json!({"layers":[camera],"parent":parent,"compensate":false})).unwrap();
    session.execute("layer.setParent", json!({"layers":[light],"parent":camera,"compensate":false})).unwrap();
    let mut doc = kition_motion_kit::MotionDocument::open("Rig premises.kmotion", session.project.to_json().as_bytes()).unwrap();
    let params = json!({"layers":[light],"camera":camera,"on":false});
    let actual = geometry_runtime_under(&mut doc, comp, "light.controlWithCamera", params.clone());
    assert_eq!(doc.execute("light.controlWithCamera", params).unwrap()["controlled"], false);
    // rig3d::control_light preserves released lights' world placement via
    // camera::layer_frame -> parent_world -> EvalCtx::world_matrix(camera).
    geometry_required_premises("light.controlWithCamera", &actual, &[geometry_null_matrix_premise(parent)]);
}

#[test]
fn geometry_auto_trace_requires_nested_render_source() {
    use kition_kit::{KitDocument, Premise};
    use kition_motion_engine::Session;
    use kition_motion_project::{ItemId, LayerId};
    use serde_json::json;

    let mut session = Session::default();
    let nested = session.execute("comp.new", json!({"name":"Nested","width":32,"height":32,"duration":2})).unwrap()["comp"].as_u64().unwrap();
    let child = session.execute("layer.newSolid", json!({"name":"Pixels","width":8,"height":8,"color":"#ff0000"})).unwrap()["layer"].as_u64().unwrap();
    let solid = session.project.comp(ItemId(nested)).unwrap().layer(LayerId(child)).unwrap().source.item().unwrap().0;
    let comp = session.execute("comp.new", json!({"name":"Main","width":32,"height":32,"duration":2})).unwrap()["comp"].as_u64().unwrap();
    let layer = session.execute("layer.addItem", json!({"item":nested})).unwrap()["layer"].as_u64().unwrap();
    let mut doc = kition_motion_kit::MotionDocument::open("Render premises.kmotion", session.project.to_json().as_bytes()).unwrap();
    let params = json!({"layer":layer,"channel":"red","blur":0,"minimumArea":0});
    let actual = geometry_runtime_under(&mut doc, comp, "layer.autoTrace", params.clone());
    assert!(doc.execute("layer.autoTrace", params).unwrap()["masks"].as_u64().unwrap() > 0, "the nested solid produces traceable pixels");
    let expected = vec![
        Premise::on(json!(nested)).fields(["comp.attrs.width", "comp.attrs.height"]).positions(Vec::<String>::new()),
        Premise::on(json!(child))
            .fields(["attrs.source", "sourceItem", "transform/opacity", "transform/position", "transform/anchor", "transform/scale", "transform/rotation"])
            .positions(["startTime", "inPoint", "outPoint"]),
        Premise::on(json!(solid)).fields(["kind", "data.color", "data.width", "data.height", "data.pixel_aspect"]).positions(Vec::<String>::new()),
    ];
    // autotrace::run -> tracking::source_frame -> Renderer::layer_input/content
    // -> precomp_source/render -> nested solid pixels. These objects are not params.
    geometry_required_premises("layer.autoTrace", &actual, &expected);
}
