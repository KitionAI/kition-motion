//! Independent handler readings for C8-motion-read. Family tables are committed before comparison.
#![allow(clippy::unwrap_used, clippy::panic)]

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;

#[path = "independent_animation.rs"]
mod animation;
#[path = "independent_geometry.rs"]
mod geometry;
#[path = "independent_layer.rs"]
mod layer;

// Pinned from the declaration after review: this reader opened acts.rs before reading these
// handlers. The source citations, dependencies and branches below come from the handlers.
pub const ROWS: &[(&str, &str)] = &[
    (
        "camera.linkFocusToPoi",
        "path=expression; resolves=comp id/numeric-string/name/active,camera resolver or first selected camera; reads=part,selection; holds=camera.source,camera.cameraOptions/focusDistance.expr; checks=selected camera,source camera,focus property exists; writes=focusDistance.expr; source=commands/focus.rs:16,41,51",
    ),
    (
        "camera.linkFocusToLayer",
        "path=expression; resolves=comp,camera resolver or selected camera,layer resolver or first other selected layer; reads=part,selection; holds=camera.source,camera.cameraOptions/focusDistance.expr,target.name; checks=camera kind,target exists,focus property exists; writes=focusDistance.expr generated using target name; source=commands/focus.rs:16,66",
    ),
    (
        "camera.setFocusToLayer",
        "path=evaluated; resolves=comp,camera resolver or selected camera,layer resolver or first other selected layer; reads=part,selection,time; holds=camera.source,camera and target world transforms,parent chains,referenced expressions,comp dimensions,camera options,camera timing,focusDistance keys/value; checks=camera kind,target exists,positive camera depth,focus property exists; writes=focusDistance.set_value_at(layer_time(current comp time)); source=commands/focus.rs:74,85",
    ),
    (
        "view.addGuide",
        "path=explicit; resolves=comp; reads=part; holds=comp.guides; checks=comp exists,valid orientation; writes=append guide; source=commands/view.rs:15",
    ),
    (
        "view.addGuide",
        "path=default; resolves=comp; reads=part; holds=comp.guides,vertical?comp.width:comp.height; checks=comp exists,valid orientation; writes=append guide at dimension/2; source=commands/view.rs:15",
    ),
    (
        "view.clearGuides",
        "path=clear; resolves=comp; reads=part; holds=comp.guides; checks=ACTIVE comp has guides,requested comp exists; writes=comp.guides clear; source=commands/view.rs:10,32",
    ),
    (
        "view.importGuides",
        "path=append; resolves=comp,path host lent file; reads=part; holds=comp.guides; checks=comp exists,host read and guides deserialize; writes=append file guides; source=commands/view.rs:50",
    ),
    (
        "view.moveGuide",
        "path=index; resolves=comp,guide zero-based index; reads=part; holds=comp.guides[index].position,guide index identity; checks=index exists,position present; writes=guide.position; source=commands/viewer_cmds.rs:189,199",
    ),
    (
        "view.removeGuide",
        "path=index; resolves=comp,guide zero-based index; reads=part; holds=comp.guides[index],guide index identity; checks=index exists; writes=remove indexed guide; source=commands/viewer_cmds.rs:189,210",
    ),
    (
        "shape.newPath",
        "path=existing-layer-space; resolves=comp,shape_tool draw_target resolver/selection; reads=part,selection,shape tool paint defaults; holds=target.source,locked,contents append group; checks=nonempty vertices,shape target,unlocked,paint valid; writes=new contents group; source=commands/viewer_cmds.rs:229,shape_tool.rs:draw_target",
    ),
    (
        "shape.newPath",
        "path=existing-comp-space; resolves=comp,shape_tool draw_target; reads=part,selection,time,shape tool paint defaults; holds=target.source,locked,contents,layer-to-comp transforms,parent chains,expressions; checks=nonempty vertices,shape target,unlocked; writes=new contents group using inverse transform; source=commands/viewer_cmds.rs:229",
    ),
    (
        "shape.newPath",
        "path=new; resolves=comp,name forces new when layer absent; reads=part,shape tool paint defaults; holds=comp dimensions,duration,frameRate; checks=nonempty vertices,paint valid; writes=new shape layer and contents; source=commands/viewer_cmds.rs:229",
    ),
    (
        "project.setProjectComment",
        "path=assign; resolves=none; reads=none; holds=project.settings.comment; checks=comment string; writes=project.settings.comment; source=commands/panels_cmds.rs:156",
    ),
    (
        "jobs.cancel",
        "path=cancel; resolves=job string or all; reads=running jobs; holds=none persisted; checks=none; writes=job cancellation flags; source=commands/panels_cmds.rs:21",
    ),
    (
        "mediaBrowser.action",
        "path=toolbar; resolves=action string; reads=browser service; holds=none persisted; checks=browser installed,action exists; writes=browser action effects; source=commands/panels_cmds.rs:80",
    ),
    (
        "mediaBrowser.addFavorite",
        "path=add; resolves=path or current browser folder or browser home; reads=browser folder,favorites; holds=none persisted; checks=browser installed; writes=browser favorites; source=commands/panels_cmds.rs:86",
    ),
    (
        "mediaBrowser.removeFavorite",
        "path=remove; resolves=path or current browser folder or browser home; reads=browser folder,favorites; holds=none persisted; checks=browser installed; writes=browser favorites; source=commands/panels_cmds.rs:86",
    ),
    (
        "mediaBrowser.import",
        "path=fetch-pending; resolves=paths/path strings; reads=browser service; holds=none persisted; checks=browser fetch pending; writes=deferred import request; source=commands/panels_cmds.rs:98",
    ),
    (
        "mediaBrowser.import",
        "path=ready; resolves=paths/path plus add_to_comp destination; reads=part; holds=import destination dimensions,timing and insertion gap when adding; checks=browser fetch ready,file import checks; writes=items and optional layers; source=commands/panels_cmds.rs:98,file.rs:add_to_comp",
    ),
    (
        "footage.open",
        "path=panel; resolves=item integer id or literal name or panel item/project selection; reads=footage panel,project selection; holds=item.kind for eligibility; checks=footage non-data or solid; writes=footage panel only; source=commands/footage_panel.rs:31,78",
    ),
    (
        "footage.setTime",
        "path=panel; resolves=current panel item; reads=footage panel,part; holds=item kind,duration,frameRate or comp frameRate for solid; checks=panel exists,time/frame present; writes=panel time; source=commands/footage_panel.rs:96",
    ),
    (
        "footage.setIn",
        "path=panel; resolves=current panel item; reads=footage panel,part; holds=item kind,duration,frameRate or comp frameRate for solid; checks=panel exists; writes=panel in and possibly clears out; source=commands/footage_panel.rs:104",
    ),
    (
        "footage.setOut",
        "path=panel; resolves=current panel item; reads=footage panel,part; holds=item kind,duration,frameRate or comp frameRate for solid; checks=panel exists; writes=panel out and possibly clears in; source=commands/footage_panel.rs:104",
    ),
    (
        "footage.clearInOut",
        "path=panel; resolves=current panel; reads=footage panel; holds=none persisted; checks=panel exists; writes=panel in/out None; source=commands/footage_panel.rs:126",
    ),
    (
        "footage.overlayEdit",
        "path=insert; resolves=item integer id/literal name/panel/project selection,comp; reads=part,time unless at,footage panel marks; holds=item.kind,name,size,duration,comp.frameRate,duration,width,height,insertion gap; checks=panel exists even explicit item,footage non-data/solid,out>in,at<duration; writes=new layer at top with source range; source=commands/footage_panel.rs:134",
    ),
    (
        "footage.rippleInsertEdit",
        "path=ripple; resolves=item integer id/literal name/panel/project selection,comp; reads=part,time unless at,footage panel marks; holds=overlay facts,layer in/out eligibility,complete crossing layers cloned,shifted layer timing; checks=panel exists even explicit item,source kind,out>in,at<duration; writes=shift later layer timing,split crossing layers,new footage layer; source=commands/footage_panel.rs:134",
    ),
    (
        "comp.vr.setView",
        "path=environment; resolves=comp u64 else active else unique environment; reads=part; holds=environment discovery source/effects/cube width/face positions/camera kinds,front orientation,all six destination orientation tracks and timing; checks=environment exists and unique fallback,orientation numbers; writes=six cameras orientation via prop.set at each face current time; source=commands/vr_editor.rs:30,97,109,123",
    ),
    (
        "comp.vr.createEnvironment",
        "path=create; resolves=comp resolver; reads=part; holds=source comp name,width,height,frameRate,duration,light source eligibility and complete copied lights; checks=comp exists; writes=eight comps and cameras/precomp layers,light copies,opens output; source=commands/app_more.rs:462",
    ),
    (
        "comp.vr.extractCubemap",
        "path=create; resolves=comp resolver; reads=part; holds=source comp name,width,height,frameRate,duration; checks=comp exists; writes=cubemap comp and precomp layer with VR effect; source=commands/app_more.rs:558",
    ),
    (
        "edit.history",
        "path=undo-redo; resolves=steps/redo count; reads=undo/redo history; holds=fields replaced by history operation; checks=count<=history length; writes=history project snapshots; source=commands/app_more.rs:55,history.rs",
    ),
    (
        "file.importRecent",
        "path=import; resolves=path or recent_footage index/default first; reads=recent footage preferences; holds=file.import dependencies; checks=recent exists,index valid,import checks; writes=imported items; source=commands/app_more.rs:82",
    ),
    (
        "file.clearRecentFootage",
        "path=clear; resolves=none; reads=recent footage preferences; holds=none persisted; checks=recent not empty; writes=preferences only; source=commands/app_more.rs:95",
    ),
    (
        "anim.applyRecentPreset",
        "path=apply; resolves=path or recent_presets index/default first,layers; reads=part,selection,recent preset preferences; holds=anim.applyPreset dependencies including target assigned properties; checks=selected layers,recent exists,index valid; writes=preset properties at current time; source=commands/app_more.rs:102",
    ),
    (
        "anim.clearRecentPresets",
        "path=clear; resolves=none; reads=recent preset preferences; holds=none persisted; checks=recent not empty; writes=preferences only; source=commands/app_more.rs:120",
    ),
    (
        "file.replaceWithLayeredComp",
        "path=replace; resolves=item u64 or project selection first; reads=project selection; holds=item.data path/codec,all layers source matching item; checks=selected footage,PSD codec or suffix,host file read,PSD importer; writes=new PSD comp,deletes footage,replaces all matching sources; source=commands/app_more.rs:140",
    ),
    (
        "file.importVanishingPoint",
        "path=disabled; resolves=none; reads=none; holds=none; checks=always disabled undocumented format; writes=none returns error; source=commands/vpe.rs:15",
    ),
    (
        "file.watchFolder",
        "path=stop; resolves=none; reads=none; holds=none persisted; checks=stop true; writes=watch_folder None; source=commands/watch_folder.rs:110",
    ),
    (
        "file.watchFolder",
        "path=watch; resolves=folder/path filesystem; reads=filesystem capability; holds=none current project; checks=filesystem allowed,directory exists; writes=watch folder state and OTHER projects rendered through separate Session; source=commands/watch_folder.rs:110",
    ),
    (
        "file.watchFolder.poll",
        "path=poll; resolves=folder or watched folder; reads=watch folder; holds=none current project; checks=watch folder set,filesystem allowed; writes=separate project render status files; source=commands/watch_folder.rs:130",
    ),
    (
        "file.installScript",
        "path=install; resolves=path host file; reads=settings store; holds=none project; checks=js/jsx extension,safe bare filename,UTF8,settings store exists; writes=installed script settings file; source=commands/scripts.rs:190",
    ),
    (
        "file.installScriptPanel",
        "path=install; resolves=path host file; reads=settings store; holds=none project; checks=js/jsx extension,safe bare filename,UTF8,settings store exists; writes=installed panel settings file; source=commands/scripts.rs:190",
    ),
    (
        "file.uninstallScript",
        "path=uninstall; resolves=bare script name; reads=settings store; holds=none project; checks=valid bare script name,installed,settings store exists; writes=remove both script and panel settings files; source=commands/scripts.rs:219",
    ),
    (
        "script.settings.save",
        "path=settings; resolves=section/key literal; reads=settings store; holds=none project; checks=bounded section/key/value,value provided; writes=script_settings section/key and settings file; source=commands/scripts.rs:83",
    ),
    ("anim.browsePresets", "path=frontend; resolves=none; reads=none; holds=none; checks=always; writes=frontend event only; source=commands/frontend.rs:99"),
    ("effect.manage", "path=frontend; resolves=none; reads=none; holds=none; checks=always; writes=frontend event only; source=commands/frontend.rs:98"),
    (
        "layer.style.options",
        "path=frontend; resolves=layer not consumed; reads=selection,part eligibility only; holds=none; checks=selected layers and active comp; writes=frontend event only; source=commands/frontend.rs:24",
    ),
    (
        "layer.openLayer",
        "path=frontend; resolves=layer not consumed; reads=selection,part eligibility only; holds=none; checks=selected layers and active comp; writes=frontend event only; source=commands/frontend.rs:97",
    ),
    (
        "project.deleteWithoutConfirmation",
        "path=frontend; resolves=none; reads=none; holds=none; checks=always; writes=frontend event only UI later dispatches project.delete; source=commands/frontend.rs:104",
    ),
    (
        "file.executeFile",
        "path=open-external; resolves=path filesystem; reads=script file permissions,confirmation preference; holds=none project; checks=filesystem allowed,path exists,script permission,confirmation or returns request; writes=OpenUrl frontend event only; source=commands/app_more.rs:182-204 proves no script dispatch",
    ),
    (
        "file.openRecent",
        "path=replace; resolves=path else recent project index or first; reads=recent project preferences; holds=current project assigned replacement; checks=recent nonempty,index exists,file.open validation; writes=replace project and recent list on missing file; source=commands/settings.rs:213",
    ),
    (
        "file.clearRecent",
        "path=preferences; resolves=none; reads=recent projects; holds=none project; checks=recent list nonempty; writes=clear preference list only; source=commands/settings.rs:235",
    ),
    (
        "file.autoSave",
        "path=host-owned; resolves=autosave destination; reads=project,path,autosave preferences; holds=project snapshot saved externally; checks=host-files-only refuses own autosaves; writes=autosave file or error; source=commands/settings.rs:246,autosave.rs",
    ),
    ("file.newFromTemplate", "path=frontend; resolves=none; reads=none; holds=none; checks=always; writes=frontend event only; source=templates.rs:935"),
    (
        "templates.create",
        "path=replace; resolves=template id builtin/user or lent path; reads=template bytes,settings store,footage/project destination defaults; holds=entire current project assigned replacement; checks=template valid,footage extraction host writes succeed; writes=replaces project,optional saveAs,opens main comp poster time; source=templates.rs:629,788",
    ),
    (
        "templates.saveAs",
        "path=export; resolves=name settings-store file; reads=part,time,entire project,footage bytes,user template names; holds=project snapshot and rendered frame dependency closure; checks=nonblank name,composition exists,overwrite eligibility; writes=template package file only; source=templates.rs:813",
    ),
    (
        "templates.delete",
        "path=remove; resolves=user template id only; reads=settings store; holds=none project; checks=user prefix,safe file,settings store exists; writes=template file removed; source=templates.rs:896",
    ),
    (
        "storage.persist",
        "path=host; resolves=storage host; reads=storage host availability; holds=none project; checks=host present; writes=storage persistence request; source=storage.rs:63",
    ),
    (
        "storage.clear",
        "path=host; resolves=what enum default diskCache; reads=storage host availability; holds=none project; checks=clearable enum,host present; writes=external browser storage removal; source=storage.rs:67",
    ),
    (
        "learn.start",
        "path=tutorial; resolves=id or tutorial alias in built-in tutorial catalogue; reads=selection for returned target; holds=none document; checks=tutorial exists; writes=tutorial editor progress; source=learn.rs:406",
    ),
    (
        "learn.step",
        "path=navigate; resolves=action,index into current tutorial; reads=tutorial progress,selection for returned target; holds=none document; checks=tutorial running,valid action/index; writes=tutorial editor progress; source=learn.rs:413",
    ),
    (
        "learn.step",
        "path=showMe; resolves=current tutorial step command sequence; reads=tutorial progress and delegated command dependencies; holds=union of delegated command dependencies; checks=step exists,each delegated execute; writes=each built-in show sequence and progress; source=learn.rs:413,step.show",
    ),
    ("learn.stop", "path=close; resolves=none; reads=none; holds=none document; checks=always; writes=tutorial progress None; source=learn.rs:437"),
    (
        "scriptWindow.click",
        "path=callback; resolves=window u64/title or unique window,widget id/#id/name/text; reads=script UI tree,enabled,callback closure; holds=dynamic script callback dependencies; checks=window/widget exist,enabled,script dispatch installed; writes=callback-dependent document and script UI; source=script_windows.rs:613,630,657",
    ),
    (
        "scriptWindow.set",
        "path=callback; resolves=window,widget,list value text to item index; reads=script UI tree,enabled,list items,callback closure; holds=dynamic script callback dependencies; checks=window/widget exist,enabled,value given,list item exists,script dispatch installed; writes=callback onChanging/onChange-dependent document and UI; source=script_windows.rs:669",
    ),
    (
        "scriptWindow.close",
        "path=callback; resolves=window id/title or unique window; reads=script UI tree,onClose callback; holds=dynamic script callback dependencies; checks=window exists,script dispatch installed; writes=callback-dependent document and UI,remembered panels; source=script_windows.rs:690",
    ),
    (
        "comp.saveFrameAsPsd",
        "path=export; resolves=comp resolver,path host destination; reads=part,time unless explicit time,render preferences,full rendering closure including nested comps,solo/visibility,timing,matte,source,effects,parent/expression references,name/blend/raw-opacity; holds=rendered dependency closure; checks=active comp exists,path present; writes=external layered PSD file only; source=commands/frame_export.rs:30,45,111,195,203",
    ),
    (
        "comp.saveFrameAsExr",
        "path=export; resolves=comp resolver,path host destination; reads=part,time unless explicit time,render preferences,full rendering closure,visible layer names; holds=rendered dependency closure; checks=active comp exists,path present; writes=external layered EXR file only; source=commands/frame_export.rs:30,45,158,195,211",
    ),
    (
        "roto.model.select",
        "path=choose; resolves=id registry string or classical; reads=model lane/config preferences; holds=none project; checks=registry task match,wait errors; writes=model lane,preferences; source=models.rs:313,segment/src/lib.rs:138",
    ),
    (
        "face.model.select",
        "path=choose; resolves=id registry string or classical; reads=model lane/config preferences; holds=none project; checks=registry task match,wait errors; writes=model lane,preferences; source=models.rs:313,segment/src/lib.rs:138",
    ),
    (
        "roto.model.install",
        "path=install; resolves=path filesystem,id registry or checksum match; reads=model lane/config; holds=none project; checks=settings directory,file read,registry task/hash validation,wait errors; writes=model file,model lane; source=models.rs:330",
    ),
    (
        "face.model.install",
        "path=install; resolves=path filesystem,id registry or checksum match; reads=model lane/config; holds=none project; checks=settings directory,file read,registry task/hash validation,wait errors; writes=model file,model lane; source=models.rs:330",
    ),
    (
        "roto.model.download",
        "path=download; resolves=id model registry; reads=model lane/config; holds=none project; checks=registry task,settings directory,not busy,download/hash success; writes=model file and background lane; source=models.rs:352",
    ),
    (
        "face.model.download",
        "path=download; resolves=id model registry; reads=model lane/config; holds=none project; checks=registry task,settings directory,not busy,download/hash success; writes=model file and background lane; source=models.rs:352",
    ),
    (
        "roto.model.remove",
        "path=remove; resolves=id registry and installed file; reads=model lane/config; holds=none project; checks=registry task,path optional; writes=remove model file/notices and active model if matching; source=models.rs:389",
    ),
    (
        "face.model.remove",
        "path=remove; resolves=id registry and installed file; reads=model lane/config; holds=none project; checks=registry task,path optional; writes=remove model file/notices and active model if matching; source=models.rs:389",
    ),
];

#[test]
fn reviewed_miscellaneous_rows_name_registered_commands() {
    let mut seen = std::collections::BTreeSet::new();
    for (command, row) in ROWS {
        assert!(kition_motion_engine::commands::find(command).is_some(), "{command}");
        assert!(seen.insert((command, row.split(';').next().unwrap())), "duplicate path {command}");
        for field in ["resolves=", "reads=", "holds=", "checks=", "writes=", "source="] {
            assert!(row.contains(field), "{command}: missing {field}");
        }
    }
}

#[test]
fn enumerate_write_commands() {
    let doc = MotionDocument::create("Independent.kmotion", 320, 180).unwrap();
    let commands = doc.commands();
    let mut writes: Vec<_> = commands.iter().filter(|command| command.mutates).map(|command| command.id.as_str()).collect();
    writes.sort_unstable();
    writes.dedup();
    println!("WRITE_COMMANDS {}", writes.len());
    for command in writes {
        println!("WRITE {command}");
    }
}

// Registry coverage is asserted by declarations_independent_r2::write_registry_inventory,
// which also requires every row's path and source. The older exposed rows are not its oracle.

#[test]
fn nested_reference_roles_have_schema_nodes() {
    let doc = MotionDocument::create("NestedReferences.kmotion", 64, 36).unwrap();
    let commands = doc.commands();
    let command = |id: &str| commands.iter().find(|command| command.id == id).unwrap();
    // link::pick_whip passes the entire target to prop_ref, including target.comp.
    let target_comp = &command("prop.pickWhip").schema.as_ref().unwrap()["target"]["properties"]["comp"];
    assert!(target_comp.is_object(), "a metadata path without a schema node is not traversed by the SDK");
    assert!(command("prop.pickWhip").objects.contains(&"target.comp"));
    // mask::vertex_groups accepts explicit {layer, mask, index} entries even though
    // the delete command's abbreviated notation only says `vertices?`.
    let vertex = &command("mask.deleteVertices").schema.as_ref().unwrap()["vertices"]["items"]["properties"];
    for field in ["layer", "mask", "index"] {
        assert!(vertex[field].is_object(), "explicit vertex {field} must have a schema node");
        assert_eq!(vertex[field]["required"], true, "vertex_groups requires {field}");
    }
    assert!(command("mask.deleteVertices").objects.contains(&"vertices[].layer"));
}
