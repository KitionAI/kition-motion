//! C8-motion-read independent handler reading, committed before declaration comparison.

pub(crate) const ROWS: &[(&str, &str)] = &[
    (
        "renderQueue.add",
        "path=append; resolves=comp explicit else active else first selected comp; reads=movie default templates/names,queue ids,comp name/dimensions/frameRate/workArea/duration for result/output path; holds=editor.activeComp/projectSelection,ROI for crop.useRoi,prefs output folder/session.path; checks=not rendering,valid settings/output/codec enums,time span,alpha support,ROI when requested; writes=new queue item; source=commands/render_queue.rs:add,apply_settings,apply_output,item_json; Session::resolve_output",
    ),
    (
        "renderQueue.remove",
        "path=remove; resolves=item stable id or 1-based index or singleton; reads=queue membership/order,full removed item; holds=render job state; checks=not rendering,item exists; writes=queue item removed; source=commands/render_queue.rs:item_index,remove",
    ),
    (
        "renderQueue.setRender",
        "path=assign-or-toggle; resolves=queue item id/index/singleton; reads=queue membership/order,item.render/status same-field lost-update; holds=render job state; checks=not rendering,item exists; writes=render,status queued/unqueued; source=commands/render_queue.rs:set_render",
    ),
    (
        "renderQueue.setRenderSettings",
        "path=settings; resolves=queue item id/index/singleton,template name; reads=current full settings to preserve fields,custom start for duration,motionBlurMode,template contents,render/status requeue,comp/result facts; holds=render job state,prefs output folder/session.path; checks=nonempty change,valid settings/span; writes=settings clone with supplied fields,status requeue; source=commands/render_queue.rs:set_render_settings,apply_settings,item_json",
    ),
    (
        "renderQueue.setOutputModule",
        "path=module-fields; resolves=queue item id/index/singleton,module 1-based,template; reads=current full module/format/channels/codec/output/crop/resize,template,render/status,postRenderAction same-field lost-update,result comp/settings/queue facts; holds=editor.ROI for crop.useRoi,prefs output folder/session.path,render job state; checks=module range,valid enum/alpha/ROI,nonempty change; writes=module clone changed fields,postRenderAction optional,status requeue; source=commands/render_queue.rs:set_output_module,apply_output,apply_om_template",
    ),
    (
        "renderQueue.setOutputModule",
        "path=height-locked-aspect; resolves=queue item,module; reads=module resize enabled/lockAspect/height,crop fields,queue.settings resolution,comp dimensions,plus output module path reads; holds=editor.ROI; checks=height given,width/preset absent; writes=module.resize.width derived from cropped render bounds plus supplied fields,status requeue; source=commands/render_queue.rs:set_output_module",
    ),
    (
        "renderQueue.setOutput",
        "path=path-and-format; resolves=queue item,module; reads=full module clone,format/channels/output,render/status,comp/result/output-path facts; holds=prefs output folder/session.path; checks=path required,module range,not rendering; writes=module.output and format/channels adjustments,status requeue; source=commands/render_queue.rs:set_output,apply_output; OutputModule::set_format",
    ),
    (
        "render.addOutputModule",
        "path=append; resolves=queue item/id/index else last; reads=primary full output module,extra output count,templates when selected,render/status,result comp/settings/output path facts; holds=editor.ROI,prefs output folder/session.path; checks=queue nonempty,valid format/options; writes=extra output module clone with unique suffix/options,status requeue; source=commands/render_queue.rs:add_output_module,apply_output",
    ),
    (
        "render.preRender",
        "path=append; resolves=comp explicit else active else first selected; reads=preRender default templates,queue ids,comp/result facts; holds=editor.activeComp/projectSelection,ROI,prefs output folder/session.path; checks=not rendering,output parameters valid; writes=new queue item postRenderAction=ImportAndReplace; source=commands/render_queue.rs:pre_render",
    ),
    (
        "render.saveCurrentPreview",
        "path=export; resolves=comp,path-derived format; reads=comp.workArea/frameRate/dimensions/background,full rendered dependency graph,project render settings; holds=exporter/footage/expr/accel/storage,prefs.switchesAffectNestedComps; checks=path/exporter/supported format; writes=exported movie file,toast; source=commands/render_queue.rs:save_current_preview; Exporter::export",
    ),
    (
        "renderQueue.move",
        "path=reorder; resolves=item id/index/singleton,to clamped index; reads=queue membership/order; holds=render job state; checks=not rendering,to required; writes=queue order; source=commands/render_queue.rs:move_item",
    ),
    (
        "renderQueue.duplicate",
        "path=clone; resolves=item id/index/singleton; reads=full source item,queue ids/order; holds=render job state; checks=not rendering,item exists; writes=clone fresh id/render=true/status queued/cleared run metadata; source=commands/render_queue.rs:duplicate",
    ),
    (
        "renderQueue.render",
        "path=start; resolves=all queued items and output modules; reads=queue.render/status/settings/modules,comp/source render dependency graph,project bitDepth/templates for filenames/output prefs,render_prefs overflow; holds=exporter/hosts,prefs autosave/output/segments/nestedSwitches,session.path,job state; checks=not already rendering,supported formats/valid output,queued items; writes=queue status/run metadata,render jobs/files,optional autosave; source=commands/render_queue.rs:render; Session::start_render,resolve_output,segments,poll_render",
    ),
    (
        "renderQueue.render",
        "path=post-render; resolves=completed item comp and output file; reads=live item.postRender/comp,proxy same-field lost-update or all comps source refs for replacement,output probe metadata; holds=editor selections,importer/services,prefs.notify; checks=done output and action enabled,probe succeeds; writes=set comp proxy or new footage item plus replace every matching comp source,selection,notifications; source=Session::poll_render_apply,post_render_action",
    ),
    (
        "renderQueue.stop",
        "path=cancel; resolves=running render job/remote ids; reads=job completion/updates,queue status for remote remaining ids; holds=renderJob/offload; checks=running job; writes=cancel signal,remote user-stopped pending updates; source=Session::stop_render",
    ),
    (
        "renderQueue.list",
        "path=poll-writes; resolves=pending render update item ids; reads=live queue membership/postRender action,notify preference,full comp/settings/modules for returned facts; holds=render job/updates/hosts; checks=completed statuses and postrender conditions; writes=queue status/start/renderTime/lastOutput,postrender proxy/import/source replacement,events; source=commands/render_queue.rs:list; Session::poll_render_apply,post_render_action",
    ),
    (
        "renderQueue.saveTemplate",
        "path=render-settings; resolves=kind,name,item/index or from template else movie default; reads=source settings,all template clone/current same-name template same-field lost-update,params settings reads,returned all template facts; holds=none; checks=kind/name,Custom reserved,source exists,params valid; writes=renderTemplates updated named render settings; source=commands/render_queue.rs:save_template; RenderTemplates::save_render_settings",
    ),
    (
        "renderQueue.saveTemplate",
        "path=output-module; resolves=kind,name,item/index/module or from template else movie default; reads=source module,all template clone/current same-name template same-field lost-update,params output reads; holds=editor.ROI for crop.useRoi; checks=kind/name,Custom reserved,module/source exists,options valid; writes=renderTemplates named module with default output naming; source=commands/render_queue.rs:save_template; RenderTemplates::save_output_module",
    ),
    (
        "renderQueue.deleteTemplate",
        "path=delete; resolves=kind,name case-insensitive; reads=all template clone,deleted template full content,builtin status; holds=none; checks=template exists,not undeletable builtin; writes=renderTemplates without custom/override; source=commands/render_queue.rs:delete_template; RenderTemplates::delete",
    ),
    (
        "renderQueue.setTemplateDefault",
        "path=assign; resolves=kind,slot,name; reads=template existence,current slot default same-field lost-update,all template clone; holds=none; checks=valid slot/name/template; writes=renderTemplates default slot; source=commands/render_queue.rs:set_template_default; RenderTemplates::set_default",
    ),
    (
        "renderQueue.applyTemplate",
        "path=render-settings; resolves=queue item,module,renderSettings template name; reads=template settings,current settings same-field lost-update,render/status,comp/result facts; holds=prefs output folder/session.path; checks=module exists,template exists; writes=item.settings,status requeue; source=commands/render_queue.rs:apply_template",
    ),
    (
        "renderQueue.applyTemplate",
        "path=output-module; resolves=queue item,module,outputModule template name; reads=template module,current module same-field lost-update/current output path and old format,render/status,comp/result facts; holds=prefs output folder/session.path; checks=module/template exists; writes=module from template preserving output path adjusted extension,status requeue; source=commands/render_queue.rs:apply_template,apply_om_template",
    ),
    (
        "renderQueue.setLog",
        "path=assign; resolves=queue item id/index/singleton; reads=log same-field lost-update,all item/comp/settings/output facts returned; holds=prefs output folder/session.path; checks=valid log,not rendering; writes=item.log; source=commands/render_queue.rs:set_log,item_json",
    ),
    (
        "renderQueue.setNotify",
        "path=assign-or-toggle; resolves=render_prefs.notify; reads=notify same-field lost-update or toggle; holds=none; checks=none; writes=notify; source=commands/render_queue.rs:set_notify",
    ),
    (
        "renderQueue.setOverflowFolders",
        "path=assign; resolves=trimmed folder list; reads=overflowFolders same-field lost-update; holds=none; checks=folders array/null; writes=overflowFolders; source=commands/render_queue.rs:set_overflow",
    ),
    (
        "file.newProject",
        "path=template-or-empty; resolves=template preference path; reads=current project being replaced for lost work,template bytes; holds=prefs.project.useTemplate/templatePath,editor state/history; checks=template readable/valid when enabled; writes=replaced whole project,path/history/editor reset,sentinel; source=file.rs:new_project; Session::replace_project",
    ),
    (
        "file.openDemoProject",
        "path=demo; resolves=demo main comp by name; reads=current project being replaced; holds=editor state/history; checks=none; writes=whole demo project,active/open comp,time=2.5; source=file.rs:demo",
    ),
    (
        "file.open",
        "path=open; resolves=file path; reads=current project being replaced,file bytes,relative footage path existence; holds=checkFootageOnOpen,editor state/history; checks=UTF8/JSON-or-XML/project validation; writes=whole project,path,recent project prefs,optional footage-check job; source=file.rs:open,resolve_relative_footage",
    ),
    (
        "file.save",
        "path=serialize; resolves=explicit path or session.path; reads=whole project; holds=session.path,savedRevision,preferences recent paths; checks=roundtrip-valid serialization,write succeeds; writes=file,path,saved snapshot/revision,recent paths; source=file.rs:save,save_to,file_text",
    ),
    (
        "file.saveAs",
        "path=serialize; resolves=required path; reads=whole project; holds=savedRevision,preferences recent paths; checks=roundtrip-valid serialization,write succeeds; writes=file,path,saved snapshot/revision,recent paths; source=file.rs:save_as,save_to",
    ),
    (
        "file.incrementAndSave",
        "path=serialize; resolves=next available incremented session.path; reads=whole project,file existence; holds=session.path; checks=project saved,serialization/write succeeds; writes=file,path,saved snapshot/revision,recent paths; source=file.rs:increment_save",
    ),
    (
        "file.revert",
        "path=reload; resolves=session.path; reads=current whole project being replaced,file bytes/relative path existence; holds=session.path,editor state/history; checks=saved path,project parses; writes=whole project and open side effects; source=file.rs:revert,open",
    ),
    (
        "file.import",
        "path=footage-sync-or-background; resolves=paths/folders/sequences,page/layer selector; reads=external file bytes/probe metadata,project ids; holds=prefs import drag mode/sequenceFPS/alpha/missingFrames,recentFootage,activeComp captured before job; checks=importAs/page/frameRate valid,probe errors per-file; writes=new footage items,projectSelection,recentFootage prefs,optional jobs/alpha-dialog/events; source=file.rs:prepare_import,Prober::probe,add_footage,import_cmd",
    ),
    (
        "file.import",
        "path=layered-compositions; resolves=paths,page; reads=PSD/vector bytes; holds=prefs.import.stillFootage/stillSeconds; checks=layered parse,mode; writes=new folder/items/comp/layers,selection,opened comp; source=file.rs:import_psd_comp,import_vector_comp; psd_import::import,vector::import_vector_comp",
    ),
    (
        "file.import",
        "path=addToComp; resolves=captured active comp,new imported items,index/time/position; reads=target comp dimensions/frameRate/duration/renderer/names/stack,imported item metadata and containment; holds=captured editor.activeComp,selectedLayers insertion,prefs layer-start/still duration; checks=layer.addItem eligibility; writes=import items plus target layers,selection,open target comp; source=file.rs:add_to_comp; layer.rs:add_item",
    ),
    (
        "file.projectSettings",
        "path=fields; resolves=project setting names; reads=all assigned settings same-field lost-update,colorEngine/workingSpace/HDR dependency,returned settings fields; holds=none; checks=enum values,gamma 2.2/2.4,OCIO linear working space; writes=supplied settings,OCIO default workingSpace/HDR; source=file.rs:project_settings",
    ),
    (
        "file.cycleBitDepth",
        "path=cycle; resolves=project bitDepth; reads=settings.bitDepth; holds=none; checks=none; writes=settings.bitDepth next; source=file.rs:cycle_depth",
    ),
    (
        "render.backend",
        "path=read-or-set; resolves=backend enum; reads=settings.gpuAcceleration,adapter name/status; holds=accel/accelNote; checks=valid backend; writes=settings.gpuAcceleration when supplied and different; source=file.rs:render_backend,backend_status",
    ),
    (
        "project.newFolder",
        "path=append; resolves=parent numeric if existing folder; reads=parent.kind; holds=none; checks=invalid parent becomes root; writes=new folder,projectSelection; source=file_more.rs:new_folder",
    ),
    (
        "file.close",
        "path=close-comp; resolves=comp; reads=comp existence; holds=editor.activeComp/openComps; checks=comp resolution; writes=delegated comp.close editor state; source=file_more.rs:close",
    ),
    (
        "file.closeProject",
        "path=replace; resolves=none; reads=current project being replaced; holds=editor state/history; checks=none; writes=empty project,path/state/history reset; source=file_more.rs:close_project",
    ),
    (
        "file.saveCopy",
        "path=serialize; resolves=required path; reads=whole project; holds=none; checks=serialization valid,write succeeds; writes=copy file only; source=file_more.rs:save_copy; file.rs:file_text",
    ),
    (
        "file.importMultiple",
        "path=delegate; resolves=file.import paths/options; reads=union file.import reads; holds=file.import context; checks=file.import checks; writes=file.import writes; source=file_more.rs:specs closure; file.rs:import_cmd",
    ),
    (
        "file.importPlaceholder",
        "path=append; resolves=none; reads=project ids; holds=none; checks=parameters default; writes=new placeholder footage,projectSelection; source=file_more.rs:import_placeholder,placeholder_footage",
    ),
    (
        "file.importSolid",
        "path=append; resolves=Solids folder by name; reads=active comp width/height for missing dimensions,Solids folder; holds=editor.activeComp; checks=dimensions >=1; writes=solid item/folder,projectSelection; source=file_more.rs:import_solid,solid_from",
    ),
    (
        "file.newCompFromSelection",
        "path=individual-or-single; resolves=projectSelection,dimensionsFrom index; reads=selected item kind/name/dimensions/rate/duration/pixelAspect/audio/containment,render templates if queued; holds=editor.projectSelection,prefs still duration and layer start; checks=nonfolders selected,valid dimensionsFrom; writes=new comps/layers,optional sequence timing/opacity and queue,open comp,selection,grouped undo; source=file_more.rs:new_comp_from_selection; layer.addItem,layer.sequence,renderQueue.add",
    ),
    (
        "file.removeUnusedFootage",
        "path=remove-unreferenced; resolves=all source item refs; reads=all layers.source,all item.kind,full deleted footage/solid items; holds=editor selections for sanitize; checks=project nonempty enablement; writes=delete unreferenced footage/solid,sanitize; source=file_more.rs:remove_unused,referenced_sources",
    ),
    (
        "file.consolidateFootage",
        "path=deduplicate; resolves=first identical footage by item order; reads=all full Footage records,path,membership,all layer.source references,deleted item metadata; holds=editor selections; checks=nonempty path,exact serialized footage equality; writes=retarget footage sources,delete duplicate items,sanitize; source=file_more.rs:consolidate",
    ),
    (
        "file.reduceProject",
        "path=closure; resolves=selected comps; reads=item.kind/parent,transitive comp layer.source,full items outside closure; holds=editor.projectSelection; checks=comp selection enablement; writes=delete items outside used closure,sanitize; source=file_more.rs:reduce,used_items",
    ),
    (
        "file.findMissing",
        "path=footage; resolves=all footage; reads=item.kind/missing; holds=editor.projectSelection same-field lost-update; checks=valid what; writes=projectSelection missing items,toast; source=file_more.rs:find_missing",
    ),
    (
        "file.findMissing",
        "path=effects-fonts; resolves=all comp layers/effect groups/text properties; reads=effect.kind and registry availability or text source font and installed font families; holds=none; checks=valid what; writes=toast,result facts only; source=file_more.rs:find_missing",
    ),
    (
        "file.collectFiles",
        "path=copy; resolves=output folder,footage paths/sequences; reads=whole project snapshot,all footage paths/sequences,source bytes; holds=session.path; checks=filesystem available,folder/write; writes=copied media and cloned project with retargeted paths,result counts/errors; source=file_more.rs:collect_files",
    ),
    (
        "file.runScript",
        "path=named-js-file; resolves=installed script name or JS file path; reads=script bytes and every script operation's reads; holds=scripting runner/permissions,console environment; checks=no jsxbin,runner available; writes=script-driven commands/model edits,report toast; source=file_more.rs:run_script,run_js; scripts.rs:run_named",
    ),
    (
        "file.runScript",
        "path=command-steps; resolves=inline/file JSON or JSONL commands; reads=union step reads; holds=script permissions when from file; checks=valid steps,no recursive scripts; writes=sequential step edits retained through failure; source=file_more.rs:run_script",
    ),
    (
        "script.run",
        "path=javascript; resolves=code/name; reads=arbitrary script model/command reads; holds=script runner/console state; checks=code required,no jsxbin; writes=script model/command edits and output; source=file_more.rs:script_run,run_js",
    ),
    (
        "file.interpretFootage",
        "path=settings; resolves=items/item numeric else selected footage or selected layer source; reads=item.kind,assigned interpretation fields same-field lost-update,nativeRate/frameRate/duration/sequence for conform,start timecode parse,all source-user stretch/startTime/outPoint when duration changes; holds=editor.projectSelection/selectedLayers fallback; checks=nonempty targets,valid interpretation; writes=footage settings/rate/duration and matching source users.outPoint across comps,result metadata; source=file_more.rs:interpret,Interpretation::apply",
    ),
    (
        "file.interpretFootage",
        "path=guess-alpha; resolves=footage targets; reads=full frame-source Footage,first-frame pixels,premulColor,plus interpretation/conform/user-retime reads; holds=editor selections,footage host; checks=partial alpha sample; writes=guessed alpha/matte plus supplied fields and duration user retime; source=file_more.rs:guess_alpha,interpret",
    ),
    (
        "file.rememberInterpretation",
        "path=copy; resolves=first numeric item else selected footage; reads=full source Footage; holds=editor.projectSelection/selectedLayers; checks=some target,footage kind; writes=editor.interpretation; source=file_more.rs:remember_interpretation",
    ),
    (
        "file.applyInterpretation",
        "path=assign; resolves=numeric items else selected footage; reads=target.kind,assigned interpretation fields same-field lost-update,target.nativeRate/frameRate; holds=editor.interpretation,projectSelection/selectedLayers; checks=remembered interpretation; writes=alpha,matte,PAR,loops,fields,invertAlpha,linearLight,preserveRgb,profile,rate/nativeRate when source conformed; source=file_more.rs:apply_interpretation",
    ),
    (
        "file.replaceFootage",
        "path=replace; resolves=first numeric item else selected footage,new source path/sequence; reads=full replaced item.kind/name,all layers.source references,new probe bytes; holds=editor.projectSelection/selectedLayers,prefs sequence rate; checks=target exists,importer/probe available; writes=item.name/kind,all matching layers.source=Footage; source=file_more.rs:replace_footage,retarget_layers",
    ),
    (
        "file.replaceWithPlaceholder",
        "path=replace; resolves=first numeric item else selected footage; reads=old footage dimensions/rate/duration for defaults,full replaced item.kind,all layers.source refs; holds=editor.projectSelection/selectedLayers; checks=item exists; writes=item.kind placeholder,matching layers.source=Footage; source=file_more.rs:replace_with_placeholder",
    ),
    (
        "file.replaceWithSolid",
        "path=replace; resolves=first numeric item else selected footage; reads=old item dimensions when width absent,active comp dimensions fallback,full replaced item.kind,all layers.source refs; holds=editor.activeComp,projectSelection/selectedLayers; checks=item exists; writes=item.kind solid,matching layers.source=Solid; source=file_more.rs:replace_with_solid",
    ),
    (
        "file.reloadFootage",
        "path=reload; resolves=numeric items else selected footage; reads=full current Footage,path/sequence/sourceLayer/page/nativeRate,source bytes/probe metadata; holds=editor.projectSelection/selectedLayers,prefs sequence rate; checks=importer exists,failed file becomes missing; writes=refreshed Footage preserving interpretation,media/layer cache purge; source=file_more.rs:reload; footage_reload.rs:reload_items,refreshed",
    ),
    (
        "footage.check",
        "path=missing-flag; resolves=items numeric else all nondata footage; reads=footage.kind/path/sequence/dimensions,external existence,missing same-field lost-update; holds=prefs sequence rate,dirty/saved state; checks=cancel and target still Footage at apply; writes=missing flag,revision/cache events,keeps saved status; source=footage_check.rs:start,apply",
    ),
    (
        "footage.check",
        "path=probe-metadata; resolves=items numeric else all nondata footage; reads=full captured Footage,external existence/probe metadata; holds=prefs sequence rate,dirty/saved state; checks=present zero dimensions nonaudio,importer; writes=whole captured-derived Footage at live item,revision/cache events; source=footage_check.rs:start,apply",
    ),
    (
        "footage.reloadChanged",
        "path=stamps-and-refresh; resolves=watched footage files and sequence frames; reads=footage paths/sequences/current full Footage,external stamps/metadata; holds=prefs.autoReloadFootage,session.footageStamps,dirty state; checks=stamp changed and readable; writes=remember stamps,refresh Footage without undo/dirtying,cache purge; source=footage_reload.rs:command,apply_stamps,reload,refreshed",
    ),
    (
        "file.revealInFinder",
        "path=file; resolves=first numeric item else selected footage; reads=item.kind/path; holds=editor.projectSelection/selectedLayers; checks=nonempty footage path; writes=open URL event; source=file_more.rs:reveal_in_finder",
    ),
    (
        "file.relinkFootage",
        "path=dry-run; resolves=all missing-path footage,recursive directory candidates; reads=full missing footage,old/new file existence,candidate metadata,sequence names; holds=importer/services; checks=filesystem,no symlinks,100000 entries,unique exact filename/compatible metadata,full old footage unchanged at apply; writes=job/results only; source=relink.rs:run,compatible,files,sequence_at",
    ),
    (
        "file.relinkFootage",
        "path=apply; resolves=all missing-path footage,recursive directory candidates; reads=full missing footage,old/new file existence,candidate metadata,sequence names; holds=importer/services; checks=unique candidate compatible,full old footage unchanged at apply,old still absent/new present; writes=footage.path/sequence/missing via old clone preserving fields; source=relink.rs:run",
    ),
    (
        "file.setProxy",
        "path=attach; resolves=numeric/name items else projectSelection,footage/comp filter; reads=item.kind/name,proxy same-field lost-update,new proxy probe; holds=editor.projectSelection; checks=targets nonempty,proxy has video; writes=item.proxy full enabled proxy; source=proxy.rs:targets,set_proxy,set_proxy_footage",
    ),
    (
        "file.setProxyNone",
        "path=remove; resolves=footage/comp targets; reads=item.kind,proxy full deleted content; holds=editor.projectSelection; checks=proxy enablement; writes=item.proxy=null; source=proxy.rs:set_proxy_none",
    ),
    (
        "file.useProxy",
        "path=assign-or-toggle; resolves=footage/comp targets with proxy; reads=item.kind,proxy presence,first proxy.enabled and assigned enabled same-field lost-update; holds=editor.projectSelection; checks=some proxy; writes=proxy.enabled; source=proxy.rs:use_proxy",
    ),
    (
        "file.interpretProxy",
        "path=interpret; resolves=footage/comp targets with proxy; reads=proxy Footage settings/rate/duration/sequence,first-frame pixels for guess,assigned fields same-field lost-update; holds=editor.projectSelection; checks=some proxy,valid interpretation; writes=proxy.footage interpretation/rate/duration; source=proxy.rs:interpret_proxy; file_more.rs:Interpretation::apply",
    ),
    (
        "file.createProxy",
        "path=still; resolves=comp/item explicit else selected comp else active; reads=comp.posterTime when inactive,frameRate,still-proxy templates,queue ids,comp name/output resolution; holds=editor.projectSelection/activeComp/currentTime; checks=kind valid; writes=new queue item one frame,proxy post-action; source=proxy.rs:create_proxy",
    ),
    (
        "file.createProxy",
        "path=movie; resolves=comp/item explicit else selected comp else active; reads=movie-proxy templates,queue ids,comp name/output resolution; holds=editor.projectSelection/activeComp; checks=kind valid; writes=new queue item full span,proxy post-action; source=proxy.rs:create_proxy",
    ),
    (
        "layer.sceneEditDetection",
        "path=markers; resolves=comp,layer; reads=source render graph/bytes,source eligibility,in/out,comp.frameRate/duration,live layer timing and markers at cut times; holds=editor.selectedLayers fallback; checks=source-backed,at least two frames,job cancel; writes=markers replacing each detected-time marker,ignores markersLocked; source=scene_detect.rs:run,analyse,apply; tracking::source_frame",
    ),
    (
        "layer.sceneEditDetection",
        "path=split; resolves=comp,layer; reads=source render graph/bytes,span/rate/duration,full live layer at apply,stack; holds=editor.selectedLayers; checks=cut still inside live span; writes=split copies fresh ids,in/out,selection; source=scene_detect.rs:apply",
    ),
    (
        "layer.sceneEditDetection",
        "path=split-precompose; resolves=comp,layer and split pieces; reads=split reads plus precompose move/adjustDuration reads; holds=editor.selectedLayers; checks=source/analysis/split/precompose checks; writes=split then each piece moved to new precomp,undo collapsed; source=scene_detect.rs:apply; layer.rs:precompose",
    ),
    (
        "file.exportLottie",
        "path=export; resolves=comp,path; reads=full comp and reachable source comps/footage,optional font outlines/source bytes; holds=services; checks=export valid,file write; writes=JSON/dotlottie file,toast; source=lottie.rs:export_lottie; kition_motion_lottie::export_comp",
    ),
    (
        "file.importLottie",
        "path=import; resolves=file path; reads=Lottie/zip bytes and linked images; holds=services; checks=parse/import valid; writes=new comp/items/layers,embedded image files,projectSelection,open comp; source=lottie.rs:import_lottie; kition_motion_lottie::import",
    ),
    (
        "effect.apply",
        "path=instantiate; resolves=comp,layers,effect registry id/name; reads=layer.source/effect bounds/source dimensions,existing effect match ids/names/order,result paths/params; holds=editor.selectedLayers; checks=nonempty targets,effects group supported,params valid/coercible; writes=new effect groups,selectedProps,lastEffect; source=effect.rs:apply,set_params; render:effect_bounds",
    ),
    (
        "effect.applyLast",
        "path=instantiate; resolves=comp,layers,last effect; reads=union effect.apply reads; holds=editor.lastEffect,selectedLayers; checks=lastEffect exists; writes=new effect groups,selectedProps,lastEffect; source=effect.rs:last,apply",
    ),
    (
        "effect.pickColor",
        "path=sample; resolves=comp,layer,effect uid/index/name or selected group,color prop uid/path; reads=effect stack order,parameter UI/type,layer input pixels before effect and rendered dependencies,prop.set current-time destination value/keys; holds=editor.selectedProps/selectedLayers,currentTime for write and sample fallback; checks=color param,x/y required,pixels nontransparent; writes=color property via prop.set at current editor time even with explicit sample time; source=effect.rs:pick_color; Renderer::layer_input",
    ),
    (
        "effect.remove",
        "path=delete; resolves=comp,effect/effects uid/index/name else selected effects; reads=effect references/membership/names/order,full removed groups; holds=editor.selectedProps,selectedLayers fallback; checks=effects exist; writes=remove groups,selectedProps pruned; source=effect.rs:effects_p,remove",
    ),
    (
        "effect.removeAll",
        "path=delete-all; resolves=comp,layers; reads=all effect groups full deleted content; holds=editor.selectedLayers; checks=layers exist; writes=clear effects; source=effect.rs:remove_all",
    ),
    (
        "effect.toggle",
        "path=assign-or-toggle; resolves=comp,layer,effect uid/index/name or selected group; reads=effect membership/name/order,enabled same-field lost-update or toggle; holds=editor.selectedProps/selectedLayers fallback; checks=effect exists; writes=effect.enabled; source=effect.rs:find_fx,toggle",
    ),
    (
        "effect.reorder",
        "path=move; resolves=comp,layer,effect uid/index/name or selected group,index; reads=effect stack order/membership/names for reference; holds=editor.selectedProps/selectedLayers fallback; checks=index required; writes=effect stack order without changing group content; source=effect.rs:find_fx,reorder",
    ),
    (
        "effect.duplicate",
        "path=clone; resolves=comp,layer,effect uid/index/name or selected group; reads=full source effect,all effect names/order; holds=editor.selectedProps/selectedLayers fallback; checks=group exists; writes=new clone fresh uids/name,selectedProps; source=effect.rs:duplicate,unique_name",
    ),
    (
        "effect.copy",
        "path=copy; resolves=effect/effects uid/index/name else selected groups; reads=selected full effect groups from active comp; holds=editor.activeComp,selectedProps/selectedLayers; checks=effect resolves; writes=effectClipboard,clear other clipboards except valueClipboard; source=effect.rs:copy,effects_p",
    ),
    (
        "effect.paste",
        "path=insert; resolves=comp,layers; reads=effects group presence,names/order,selected effect position; holds=editor.effectClipboard,selectedProps,selectedLayers; checks=clipboard/targets nonempty; writes=cloned groups fresh uids/names,selectedProps; source=effect.rs:paste",
    ),
    (
        "effect.reset",
        "path=defaults; resolves=comp,layer,effect uid/index/name or selected group; reads=effect kind/name,property presence/keys/value same-field lost-update,layer timing/effect bounds/source dimensions; holds=editor.currentTime,selectedProps/selectedLayers; checks=known effect; writes=each existing effect parameter default at layer time; source=effect.rs:reset",
    ),
    (
        "effect.plugins.load",
        "path=load; resolves=path or sorted folder wasm files; reads=plugin bytes/manifest/registry; holds=pluginLoader; checks=loader available,filesystem for folder,per-file load validation; writes=plugin registry,toast/results; source=effect.rs:plugins_load",
    ),
    // Pinned from the declaration after review: later families followed guard-summary exposure.
    // Only the preceding 87 rows (56 registry writes) were committed fully blind in 41f36275.
    (
        "edit.undo",
        "path=scoped; resolves=scope person/agent or caller; reads=scope undo stack,current project,replicated pending changes; holds=editor selections/history; checks=valid scope; writes=scoped undo patches,redo stack,selection sanitization; source=edit.rs:undo; Session::undo,in_scope",
    ),
    (
        "edit.redo",
        "path=scoped; resolves=scope person/agent or caller; reads=scope redo stack,current project,replicated pending changes; holds=editor selections/history; checks=valid scope; writes=scoped redo patches,undo stack,selection sanitization; source=edit.rs:redo; Session::redo,in_scope",
    ),
    (
        "edit.duplicate",
        "path=effects; resolves=selected effect groups; reads=selection group membership,effect full content,names/order; holds=editor.selectedProps when layers absent; checks=selected effects nonempty; writes=delegated duplicated effect groups; source=edit.rs:duplicate; effect.rs:duplicate",
    ),
    (
        "edit.duplicate",
        "path=contents; resolves=selected shape contents; reads=selected group content,target sibling names/order; holds=editor.selectedProps when layers absent; checks=shape items selected; writes=duplicated contents groups; source=edit.rs:duplicate; prop_groups.rs:duplicate_contents",
    ),
    (
        "edit.duplicate",
        "path=layers; resolves=comp,layers; reads=full target layers,comp layer names/order,parent/matte membership; holds=editor.selectedLayers; checks=target exists; writes=cloned layers fresh ids/names,relinked parent/matte,selection; source=edit.rs:duplicate,reid,relink_copies",
    ),
    (
        "edit.clear",
        "path=text; resolves=text editing target; reads=text document and selection; holds=editor.textEdit when layers absent; checks=text edit active; writes=delegated text deletion; source=edit.rs:delete; text_edit.rs:delete",
    ),
    (
        "edit.clear",
        "path=time-remap; resolves=selected timeRemap; reads=selected properties,timeRemap keys,selected key membership,layer.source; holds=editor.selectedProps,selectedKeys when layers absent; checks=all remap keys selected; writes=remove timeRemap,selection; source=edit.rs:delete; layer_time.rs:selected_time_remap,disable_time_remap",
    ),
    (
        "edit.clear",
        "path=keys-vertices-pins; resolves=selected keys or loose mask vertices or puppet pins; reads=selected target membership/full deleted content; holds=editor.selectedKeys,selectedVertices,selectedProps when layers absent; checks=branch priority keys then vertices then pins; writes=delegated keys/vertices/pins deletion; source=edit.rs:delete; keys.delete,mask.deleteVertices,puppet.removePin",
    ),
    (
        "edit.clear",
        "path=effects-groups; resolves=selected effects or masks/contents/animators/trackers; reads=selected groups full content; holds=editor.selectedProps when layers absent; checks=effects priority before groups; writes=remove selected groups; source=edit.rs:delete; effect.remove,prop_groups.rs:remove_selected",
    ),
    (
        "edit.clear",
        "path=layers; resolves=comp,layers; reads=full removed layers,locked,target users parent/matte links,orphan transforms/ancestor evaluation; holds=editor.selectedLayers,currentTime,other selection; checks=not all locked; writes=remove layers,orphan parent and compensated transform,matte users links,selection; source=edit.rs:delete; layer.rs:parent_fixes",
    ),
    (
        "edit.copy",
        "path=text; resolves=text editing target; reads=text document/selection; holds=editor.textEdit when layers absent; checks=text editing; writes=text clipboard; source=edit.rs:copy; text_edit.rs:copy",
    ),
    (
        "edit.copy",
        "path=properties; resolves=selected properties or transform group leaves; reads=property values/keys,match paths,selected keys; holds=editor.selectedProps/selectedKeys when layers absent; checks=leaf properties nonempty; writes=cleared clipboards,value clipboard,key clipboard; source=edit.rs:copy_properties",
    ),
    (
        "edit.copy",
        "path=keys-effects-contents; resolves=selected keys/effects/contents; reads=selected full keyframes/effect groups/contents; holds=editor.selectedKeys/selectedProps when layers absent; checks=priority keys then effects then contents; writes=corresponding clipboard,other clipboard clears; source=edit.rs:copy; keys.copy,effect.copy,prop_groups.rs:copy_contents",
    ),
    (
        "edit.copy",
        "path=layers; resolves=comp,layers; reads=full target layers in comp order; holds=editor.selectedLayers; checks=comp exists; writes=layer clipboard,clears other clipboards; source=edit.rs:copy",
    ),
    (
        "edit.copyWithPropertyLinks",
        "path=properties; resolves=selected leaf props; reads=layer/comp names,property name/match paths; holds=editor.selectedProps; checks=supported property roots; writes=link clipboard,clear other clipboards; source=edit.rs:copy_links,link_expression",
    ),
    (
        "edit.copyWithPropertyLinks",
        "path=layers; resolves=comp,layers; reads=full target layers,transform/effect property staticOnly/holdOnly,layer/comp names,property paths; holds=editor.selectedLayers; checks=no leaf props selected; writes=layer clipboard with linked expressions,clear other clipboards; source=edit.rs:copy_links",
    ),
    (
        "edit.copyWithRelativePropertyLinks",
        "path=properties; resolves=selected leaf props; reads=layer names,property name/match paths; holds=editor.selectedProps; checks=supported property roots; writes=relative link clipboard; source=edit.rs:copy_links,link_expression",
    ),
    (
        "edit.copyWithRelativePropertyLinks",
        "path=layers; resolves=comp,layers; reads=full target layers,transform/effect staticOnly/holdOnly,names/property paths; holds=editor.selectedLayers; checks=no leaf props selected; writes=layer clipboard with relative expressions; source=edit.rs:copy_links",
    ),
    (
        "edit.copyExpressionOnly",
        "path=selected; resolves=selected leaf props; reads=selected property expression and match paths; holds=editor.selectedProps; checks=at least one expression; writes=expression clipboard,clear other clipboards; source=edit.rs:copy_expression_only",
    ),
    (
        "edit.cut",
        "path=text-keys-contents; resolves=selected text/keys/shape contents; reads=full copied/deleted content; holds=editor.textEdit/selectedKeys/selectedProps when layers absent; checks=text then keys then contents without selected effects; writes=clipboard and delegated deletion; source=edit.rs:cut",
    ),
    (
        "edit.cut",
        "path=copy-then-clear; resolves=comp,layers or selected effect/groups; reads=union edit.copy and edit.clear branch reads; holds=editor selections and clipboards; checks=copy/delete eligibility; writes=copy clipboard,clear targets; source=edit.rs:cut,copy,delete",
    ),
    (
        "edit.paste",
        "path=text; resolves=text editing target; reads=text evaluated document; holds=editor.textEdit,text clipboard/system text; checks=text edit active; writes=delegated pasted text; source=edit.rs:paste; text_edit.rs:paste",
    ),
    (
        "edit.paste",
        "path=links; resolves=active comp,selected layers,clipboard match paths; reads=target property presence,expression same-field lost-update; holds=editor.linkClipboard,selectedLayers; checks=nonempty selection; writes=property expressions; source=edit.rs:paste_links",
    ),
    (
        "edit.paste",
        "path=values; resolves=active comp,selected layers,clipboard property paths; reads=layer.locked,timing,property type/value/keys; holds=editor.valueClipboard,keyClipboard,clipIsKeys,currentTime,selectedLayers; checks=nonempty selection,coercible values; writes=property at time,optional keys.paste; source=edit.rs:paste_properties",
    ),
    (
        "edit.paste",
        "path=keys-effects-contents; resolves=delegated clipboard targets; reads=target property/group membership and insertion reads; holds=editor.keyClipboard/effectClipboard/contentsClipboard,clipIsKeys,selections,currentTime; checks=priority keys then effects then contents; writes=delegated pasted keys/effects/contents; source=edit.rs:paste",
    ),
    (
        "edit.paste",
        "path=layers; resolves=comp; reads=target layer names/order,parent/matte membership; holds=editor.clipboard,selectedLayers insertion; checks=comp exists; writes=cloned clipboard layers with fresh ids,names/relinked links,selection; source=edit.rs:paste,reid,relink_copies",
    ),
    (
        "edit.pasteReversedKeyframes",
        "path=reverse-clipboard; resolves=keys.paste targets; reads=union delegated keys.paste reads; holds=editor.keyClipboard,selectedLayers/Props,currentTime; checks=key paste eligibility; writes=mirrored keys with swapped easing/tangents,clipboard restored; source=edit.rs:paste_reversed",
    ),
    (
        "edit.splitLayer",
        "path=inside-span; resolves=comp,layers; reads=full target layers,inPoint/outPoint,stack; holds=editor.currentTime,selectedLayers,prefs.general.createSplitLayersAbove; checks=time strictly inside layer span; writes=original outPoint,tail copy inPoint,fresh ids,stack,selection; source=edit.rs:split",
    ),
    (
        "edit.liftWorkArea",
        "path=span-cases; resolves=comp,layers else all; reads=comp.workArea,layer.locked,timing,full layer when split/deleted; holds=editor.selectedLayers; checks=before/after/contained/spanning/overlap branches; writes=remove contained,trim overlaps,split spanning,selection; source=edit.rs:work_area_cut",
    ),
    (
        "edit.extractWorkArea",
        "path=span-cases; resolves=comp,layers else all; reads=comp.workArea,layer.locked,timing,full layer when split/deleted; holds=editor.selectedLayers; checks=before/after/contained/spanning/overlap branches; writes=lift changes plus later/tail timing shifted by gap,selection; source=edit.rs:work_area_cut",
    ),
    (
        "edit.selectAll",
        "path=text-pins-layers; resolves=active comp or text/pin mode; reads=comp.hideShy,layer.locked/shy,membership,pin kind; holds=editor.textEdit,selectedProps/pins; checks=branch text then pins then selectable layers; writes=editor selection; source=edit.rs:select_all",
    ),
    (
        "edit.deselectAll",
        "path=clear; resolves=none; reads=none; holds=editor selections; checks=none; writes=selectedLayers/Props/Keys/Vertices cleared; source=edit.rs:deselect_all",
    ),
    (
        "edit.label",
        "path=keys; resolves=keys explicit or selected; reads=key label same-field lost-update; holds=editor.selectedKeys,preferences label mapping; checks=key target mode; writes=key labels; source=edit.rs:label; keys_more.rs:label_keys",
    ),
    (
        "edit.label",
        "path=layers-items; resolves=comp,layers; reads=assigned layer/item label same-field lost-update; holds=editor.selectedLayers,projectSelection,prefs label mapping; checks=known label,items only if layer ids empty; writes=layer/item labels; source=edit.rs:label",
    ),
    (
        "edit.selectLabelGroup",
        "path=layers-or-items; resolves=active comp; reads=selected and all candidate labels; holds=editor.selectedLayers,projectSelection; checks=item branch when selected layer labels empty; writes=selection of same-label layers/items; source=edit.rs:select_label_group",
    ),
    ("edit.purgeUndo", "path=clear; resolves=none; reads=none; holds=editor.history; checks=none; writes=history cleared,toast; source=edit.rs:purge"),
    (
        "edit.purge",
        "path=cache-kind; resolves=cache kind; reads=disk cache stats/storage clear response; holds=editor.snapshot/viewer flag,cache handles; checks=kind membership controls purge; writes=cache clears,snapshot flag,purge event,toast; source=edit.rs:purge_caches",
    ),
    (
        "edit.editOriginal",
        "path=selected-footage; resolves=first selected layer source else first project item; reads=layer.source,item.kind/path; holds=editor.selectedLayers,projectSelection,activeComp; checks=nonempty footage path; writes=open URL event; source=edit.rs:original_path,edit_original",
    ),
    (
        "edit.history.goto",
        "path=steps; resolves=signed history step count; reads=current history/project; holds=editor.history; checks=stops at history end; writes=undo/redo project patches/history,selection sanitized,result history current label/index; source=edit.rs:history_goto; Session::undo,redo",
    ),
    (
        "edit.history.goto",
        "path=id-index; resolves=history id or index; reads=history tree/current project; holds=editor.history; checks=valid index/id; writes=project/history line and branches,selection sanitized; source=edit.rs:history_goto; Session::goto_history",
    ),
    (
        "comp.new",
        "path=create; resolves=unique project item name; reads=all item names; holds=prefs.threeD.defaultRenderer; checks=frameRate 0.001..1000; writes=new comp,projectSelection,openComps/activeComp by default; source=comp.rs:new_comp,apply_settings",
    ),
    (
        "comp.settings",
        "path=fields; resolves=comp; reads=assigned comp/name fields same-field lost-update,old dimensions,frameRate for startTimecode/duration,old duration/workArea,renderer and text/shape geometry presence; holds=editor.activeComp fallback; checks=valid frameRate; writes=supplied settings,name,workArea clamping,geometryOptions for advanced renderer; source=comp.rs:settings,apply_settings; model3d.rs:sync_geometry_options",
    ),
    (
        "comp.settings",
        "path=resize; resolves=comp; reads=old width/height,all layer.parent,unparented position values/keys/separation; holds=editor.activeComp fallback; checks=dimensions clamped; writes=width/height,unparented position values/all keys offset by anchor; source=comp.rs:apply_settings; layer.rs:offset_position",
    ),
    (
        "comp.open",
        "path=open; resolves=comp-or-item numeric/name else first selected comp; reads=item name/kind,comp existence; holds=editor.projectSelection,openComps,activeComp; checks=comp exists; writes=openComps,activeComp,selections cleared on switch,currentTrack,event; source=comp.rs:open; Session::open_comp",
    ),
    (
        "comp.close",
        "path=close; resolves=comp; reads=comp existence; holds=editor.openComps,activeComp; checks=comp resolution; writes=openComps,activeComp last fallback,selectedLayers/Keys/Props cleared when active; source=comp.rs:close",
    ),
    (
        "comp.workArea",
        "path=explicit; resolves=comp; reads=comp.workArea same-field lost-update,duration,frameRate; holds=editor.activeComp fallback; checks=ordered minimum-frame span; writes=workArea; source=comp.rs:work_area,keep_work_area_ordered",
    ),
    (
        "comp.workArea",
        "path=begin-end-at-time; resolves=comp; reads=comp.workArea,duration,frameRate; holds=editor.currentTime; checks=ordered minimum-frame span; writes=workArea; source=comp.rs:work_area",
    ),
    (
        "comp.setSwitch",
        "path=assign-or-toggle; resolves=comp,switch field; reads=assigned hideShy/motionBlur/frameBlending/draft3d same-field lost-update or toggle; holds=editor.activeComp fallback; checks=known switch; writes=comp switch; source=comp.rs:comp_switch",
    ),
    (
        "comp.addMarker",
        "path=append; resolves=comp,time or frame; reads=comp.frameRate for frame,marker order for sorting; holds=editor.currentTime fallback; checks=comp exists; writes=new marker; source=comp.rs:add_marker",
    ),
    (
        "comp.setPosterTime",
        "path=assign; resolves=comp; reads=comp.posterTime same-field lost-update; holds=editor.currentTime; checks=comp exists; writes=posterTime; source=comp.rs:poster",
    ),
    (
        "comp.trimToWorkArea",
        "path=trim; resolves=comp; reads=comp.workArea,duration,frameRate,all layer timing; holds=editor.activeComp fallback; checks=workArea minimum frame inside comp; writes=all layer timing,comp.duration/workArea; source=comp.rs:trim_to_wa",
    ),
    (
        "comp.cropToRegionOfInterest",
        "path=crop; resolves=comp; reads=comp.width/height same-field lost-update,layer.parent,unparented positions/all keys/separation,guides position/orientation; holds=editor.regionOfInterest; checks=ROI exists; writes=comp dimensions,unparented positions,guides,ROI clear; source=comp_more.rs:crop_to_roi,crop",
    ),
    (
        "comp.cropToLayerBounds",
        "path=crop; resolves=comp,layers; reads=selected content bounds/transforms/ancestor evaluation,comp dimensions,all layer.parent/unparented positions/all keys,guides; holds=editor.currentTime,selectedLayers; checks=visible bounds nonempty; writes=comp dimensions,unparented positions,guides,ROI clear; source=comp_more.rs:crop_to_layers,crop",
    ),
    (
        "comp.saveFrameAs",
        "path=queue; resolves=comp; reads=comp.frameRate,still default render/output templates,queue ids,comp name/output path resolution; holds=editor.currentTime fallback; checks=comp exists; writes=new queue item with custom one-frame span; source=comp_more.rs:queue_frame",
    ),
    (
        "comp.saveFrameAs",
        "path=write; resolves=comp,time; reads=rendered composition dependency graph,background; holds=editor.currentTime fallback; checks=path required,PNG encoding/write success; writes=file bytes,toast; source=comp_more.rs:save_frame; Session::render",
    ),
    (
        "comp.responsiveTime",
        "path=intro; resolves=comp; reads=marker ordering; holds=editor.currentTime; checks=time>0; writes=new protected marker 0..time; source=comp_more.rs:responsive_time",
    ),
    (
        "comp.responsiveTime",
        "path=outro; resolves=comp; reads=comp.duration,marker ordering; holds=editor.currentTime; checks=duration>time; writes=new protected marker time..duration; source=comp_more.rs:responsive_time",
    ),
    (
        "comp.responsiveTime",
        "path=workArea; resolves=comp; reads=comp.workArea,marker ordering; holds=editor.activeComp fallback; checks=nonempty span; writes=new protected workArea marker; source=comp_more.rs:responsive_time",
    ),
    (
        "layer.create",
        "path=vector; resolves=comp,layers,source item; reads=source name,source,footage path/layer/page,source transform/timing/switches/parent/matte,comp dimensions/duration/names/stack,source video same-field lost-update,vector bytes; holds=editor.selectedLayers; checks=vector footage,parse; writes=generated shape/image layers,source video=false,selection,image files/import items; source=create.rs:create; vector.rs:shapes_from_vector,image_layer",
    ),
    (
        "layer.create",
        "path=text-shapes; resolves=comp,layers; reads=source text geometry/fonts evaluated at time,source transform/timing/switches/links,name,comp dimensions/duration/names/stack,source video; holds=editor.currentTime,selectedLayers; checks=text source,glyphs nonempty; writes=new shape layer,source video=false,selection; source=create.rs:create; vector.rs:shapes_from_text",
    ),
    (
        "layer.create",
        "path=text-masks; resolves=comp,layers; reads=source evaluated text/transform/ancestor dependencies,timing/name,comp dimensions/duration/names/stack,source video; holds=editor.currentTime,selectedLayers; checks=text source,glyphs nonempty; writes=Solids folder/solid,new masked layer,source video=false,selection; source=create.rs:create; vector.rs:text_outlines_in_comp,masks_layer",
    ),
    (
        "layer.create",
        "path=editable-text; resolves=comp,layers,footage layer selector; reads=full replaced source layer,footage.path/layer,PSD text bytes,comp dimensions/duration/names/stack; holds=editor.selectedLayers; checks=PSD layer contains text; writes=replace source with editable text layer fresh ids,selection; source=create.rs:create; psd_import.rs:editable_text",
    ),
    (
        "project.select",
        "path=replace-add; resolves=items numeric/name; reads=item names/membership; holds=editor.projectSelection for add; checks=items exist; writes=projectSelection; source=project_items.rs:select,item_ref",
    ),
    (
        "project.rename",
        "path=assign; resolves=item numeric/name else first selected even after failed explicit ref; reads=item.name same-field lost-update; holds=editor.projectSelection fallback; checks=nonempty trimmed name,item exists; writes=item.name; source=project_items.rs:rename",
    ),
    (
        "project.move",
        "path=assign; resolves=items/item or projectSelection,folder id/name/null; reads=target.parent same-field lost-update,folder.kind/ancestor parent chain; holds=editor.projectSelection fallback; checks=folder kind,no cycle/depth>256; writes=target.parent; source=project_items.rs:move_items,is_within",
    ),
    (
        "project.setLabel",
        "path=assign; resolves=items/item or projectSelection; reads=item.label same-field lost-update; holds=editor.projectSelection,prefs label mapping; checks=label valid; writes=item.label; source=project_items.rs:set_label",
    ),
    (
        "project.setComment",
        "path=assign; resolves=items/item or projectSelection; reads=item.comment same-field lost-update; holds=editor.projectSelection; checks=comment required,item exists; writes=item.comment; source=project_items.rs:set_comment",
    ),
    (
        "project.delete",
        "path=delete-tree-and-users; resolves=items/item or projectSelection; reads=all item ancestry for descendants,full deleted items/layers,all layer.source/parent/matte refs,queue.comp refs; holds=editor.projectSelection,editor selections; checks=target items exist; writes=remove descendants/source users/queue refs,clear parent/matte refs,sanitize editor; source=project_items.rs:delete,doomed_items",
    ),
    (
        "project.duplicate",
        "path=clone; resolves=items/item or projectSelection; reads=full source item,all item names; holds=editor.projectSelection; checks=not folder; writes=new items,comp layers fresh ids/uids,parent/matte remapped,selection; source=project_items.rs:duplicate",
    ),
    (
        "engine.batch",
        "path=success; resolves=step commands,params with prior result substitutions; reads=union of sequential inner reads; holds=editor project/history/state and inner context; checks=no nested batch/runScript/script.run,valid references,execute_checked eligibility; writes=inner edits,folded undo step,inner journals only; source=batch.rs:batch,substitute,fold",
    ),
    (
        "engine.batch",
        "path=failure-atomic; resolves=sequential steps and references; reads=inner command reads through failed step; holds=snapshot project/history/state; checks=atomic default true; writes=rollback project/history/state,sanitize,bump; source=batch.rs:batch",
    ),
    (
        "engine.batch",
        "path=failure-nonatomic; resolves=sequential steps and references; reads=inner command reads through failed step; holds=snapshot project/history/state; checks=atomic false; writes=prior successful edits retained,undo folded; source=batch.rs:batch,fold",
    ),
    (
        "layer.newSolid",
        "path=create; resolves=comp; reads=comp.width,height,pixelAspect,duration,renderer,layer-names; holds=editor.selectedLayers for insertion; checks=comp exists,pixelAspect positive; writes=Solids folder,item,new layer,selection; source=layer.rs:new_solid_like,insert_layer; build.rs:layer",
    ),
    (
        "layer.newAdjustment",
        "path=create; resolves=comp; reads=comp.width,height,pixelAspect,duration,renderer,layer-names; holds=editor.selectedLayers for insertion; checks=comp exists,pixelAspect positive; writes=Solids folder,item,new adjustment layer,selection; source=layer.rs:new_solid_like,insert_layer; build.rs:layer",
    ),
    (
        "layer.newNull",
        "path=create; resolves=comp; reads=comp.width,height,duration,renderer,layer-names; holds=editor.selectedLayers for insertion; checks=comp exists; writes=new null layer,selection; source=layer.rs:new_simple; build.rs:layer",
    ),
    (
        "layer.newText",
        "path=create; resolves=comp; reads=comp.width,height,duration,renderer,layer-names; holds=editor.selectedLayers for insertion; checks=text attributes,position finite; writes=new text layer,selection,textEdit when edit; source=layer.rs:new_text,insert_layer; build.rs:layer",
    ),
    (
        "layer.newShape",
        "path=create; resolves=comp; reads=comp.width,height,duration,renderer,layer-names; holds=editor.selectedLayers for insertion; checks=size,paint; writes=new shape layer,contents,selection; source=layer.rs:new_shape; shape_tool.rs:new_shape_layer,add_to_contents",
    ),
    (
        "layer.addItem",
        "path=comp-footage-solid; resolves=comp,item id-or-name-or-projectSelection,index; reads=item.name,kind,dimensions,duration,audio,footage.kind,loop-independent; holds=editor.projectSelection fallback,selectedLayers insertion,currentTime default,prefs.import.stillFootage,stillSeconds,prefs.general.createLayersAtCompStart; checks=item exists,not folder-or-data,comp containment cycle,index,position; writes=new layer,selection; source=layer.rs:add_item; build.rs:layer",
    ),
    (
        "layer.addItem",
        "path=model; resolves=comp,item id-or-name-or-projectSelection,index; reads=item.name,kind,model bytes/bounds/animations,comp.height,width,duration,frameRate,renderer,layer-names; holds=editor.projectSelection,selectedLayers insertion; checks=model footage,index,position; writes=new model layer,comp.renderer,geometryOptions on text/shape,selection; source=layer.rs:add_item; model3d.rs:new_model,use_advanced_3d",
    ),
    (
        "layer.select",
        "path=replace; resolves=comp,layers-or-layer id/name/place; reads=comp layer membership,names/order for references; holds=editor.selectedVertices; checks=unresolved references omitted; writes=editor.selectedLayers,selectedProps,selectedKeys,selectedVertices; source=layer.rs:select",
    ),
    (
        "layer.select",
        "path=add-toggle; resolves=comp,layers-or-layer id/name/place; reads=comp layer membership,names/order for references; holds=editor.selectedLayers; checks=unresolved references omitted; writes=editor.selectedLayers; source=layer.rs:select",
    ),
    (
        "layer.selectNext",
        "path=step; resolves=active comp; reads=comp.layers order,hideShy,layer.locked,shy; holds=editor.selectedLayers; checks=selectable next or current,empty no-op; writes=editor.selectedLayers; source=layer.rs:select_step",
    ),
    (
        "layer.selectPrevious",
        "path=step; resolves=active comp; reads=comp.layers order,hideShy,layer.locked,shy; holds=editor.selectedLayers; checks=selectable previous or current,empty no-op; writes=editor.selectedLayers; source=layer.rs:select_step",
    ),
    (
        "layer.setSwitch",
        "path=assign; resolves=comp,layers,switch field; reads=first targeted switch for validity,assigned switch same-field lost-update,threeD transform when disabling; holds=editor.selectedLayers fallback; checks=known switch; writes=targeted switch,3D Z values/keys and orientation/rotationX/rotationY values/keys/expressions when flattening; source=layer.rs:set_switch,set_switch_value,drop_3d_values",
    ),
    (
        "layer.setSwitch",
        "path=toggle; resolves=comp,layers,switch field; reads=first targeted switch,assigned switch same-field lost-update,threeD transform when disabling; holds=editor.selectedLayers fallback; checks=known switch; writes=targeted switch,3D transform flattening; source=layer.rs:set_switch,drop_3d_values",
    ),
    (
        "layer.rename",
        "path=assign; resolves=comp,layer; reads=layer.name same-field lost-update; holds=editor.selectedLayers fallback; checks=name required; writes=layer.name; source=layer.rs:rename",
    ),
    (
        "layer.setComment",
        "path=assign; resolves=comp,layers; reads=layer.comment same-field lost-update; holds=editor.selectedLayers fallback; checks=comment required; writes=layer.comment; source=layer.rs:set_comment",
    ),
    (
        "layer.setBlendMode",
        "path=assign; resolves=comp,layers; reads=layer.blendMode same-field lost-update; holds=editor.selectedLayers fallback; checks=valid mode or nonzero step; writes=layer.blendMode; source=layer.rs:blend",
    ),
    (
        "layer.setBlendMode",
        "path=step; resolves=comp,layers; reads=layer.blendMode; holds=editor.selectedLayers fallback; checks=nonzero step; writes=layer.blendMode next/previous; source=layer.rs:blend",
    ),
    (
        "layer.setTrackMatte",
        "path=set; resolves=comp,layer,matte reference; reads=layer.trackMatte,matte.video same-field lost-update; holds=editor.selectedLayers fallback; checks=matte exists,not self; writes=layer.trackMatte,matte.video=false; source=layer.rs:track_matte",
    ),
    (
        "layer.setTrackMatte",
        "path=clear; resolves=comp,layer; reads=layer.trackMatte same-field lost-update; holds=editor.selectedLayers fallback; checks=layer exists; writes=layer.trackMatte=null; source=layer.rs:track_matte",
    ),
    (
        "layer.setParent",
        "path=uncompensated; resolves=comp,layers,parent reference; reads=target.parent same-field lost-update,parent ancestor membership/parent links; holds=editor.selectedLayers fallback; checks=no self/cycle/dangling ancestor,4096 visits,1048576 work; writes=target.parent; source=layer.rs:set_parent,check_parent_chain",
    ),
    (
        "layer.setParent",
        "path=compensated-2d; resolves=comp,layers,parent reference; reads=target.parent,threeD,transform values/keys,parent and ancestor evaluated transforms/source pixelAspect; holds=editor.selectedLayers,currentTime,expression/footage hosts; checks=parent chain valid,new matrix invertible; writes=parent,position or separated XY values/keys,rotation values/keys,scale values/keys; source=layer.rs:set_parent,parent_fixes,ParentFix::apply",
    ),
    (
        "layer.setParent",
        "path=compensated-3d; resolves=comp,layers,parent reference; reads=target.parent,source,autoOrient,transform values/keys/expressions/eases/tangents,parent ancestor transforms/pixelAspect; holds=editor.selectedLayers,currentTime,expression/footage hosts; checks=parent chain valid,finite inverse,uniform pose/static orientation for pose adjustment; writes=parent,position XYZ values/keys/tangents/eases,orientation and scale/eases when pose permits; source=layer.rs:parent_fixes,ParentFix::apply",
    ),
    (
        "layer.timing",
        "path=moveInToTime; resolves=comp,layers; reads=layer.startTime,inPoint,outPoint,comp.frameRate; holds=editor.currentTime,selectedLayers; checks=comp exists; writes=startTime,inPoint,outPoint shifted; source=layer.rs:timing",
    ),
    (
        "layer.timing",
        "path=moveOutToTime; resolves=comp,layers; reads=layer.startTime,inPoint,outPoint,comp.frameRate; holds=editor.currentTime,selectedLayers; checks=comp exists; writes=startTime,inPoint,outPoint shifted; source=layer.rs:timing",
    ),
    (
        "layer.timing",
        "path=trimInToTime; resolves=comp,layers; reads=layer.inPoint same-field lost-update,outPoint,comp.frameRate; holds=editor.currentTime,selectedLayers; checks=minimum one frame; writes=inPoint; source=layer.rs:timing",
    ),
    (
        "layer.timing",
        "path=trimOutToTime; resolves=comp,layers; reads=layer.outPoint same-field lost-update,inPoint,comp.frameRate; holds=editor.currentTime,selectedLayers; checks=minimum one frame; writes=outPoint; source=layer.rs:timing",
    ),
    (
        "layer.timing",
        "path=set; resolves=comp,layers; reads=assigned startTime/inPoint/outPoint same-field lost-update,all timing for delta/start shift,opposite trim endpoint,comp.frameRate; holds=editor.selectedLayers; checks=snapped frame bounds; writes=only supplied timing plus endpoints shifted by delta/start; source=layer.rs:timing",
    ),
    (
        "layer.slip",
        "path=source-span; resolves=comp,layers; reads=layer.source,timeRemap presence,startTime,inPoint,outPoint,stretch,comp.frameRate,source duration,footage.kind,loopCount; holds=editor.selectedLayers; checks=delta/frames required,nonstill footage/precomp without remap,coverable span,one eligible; writes=startTime; source=layer.rs:slip",
    ),
    (
        "layer.slipBack",
        "path=source-span; resolves=comp,layers; reads=layer.source,timeRemap presence,startTime,inPoint,outPoint,stretch,comp.frameRate,source duration,footage.kind,loopCount; holds=editor.selectedLayers; checks=eligible source span; writes=startTime by clamped -1 frame; source=layer.rs:slip",
    ),
    (
        "layer.slipForward",
        "path=source-span; resolves=comp,layers; reads=layer.source,timeRemap presence,startTime,inPoint,outPoint,stretch,comp.frameRate,source duration,footage.kind,loopCount; holds=editor.selectedLayers; checks=eligible source span; writes=startTime by clamped +1 frame; source=layer.rs:slip",
    ),
    (
        "layer.arrange",
        "path=step; resolves=comp,layers; reads=comp.layers order,layer.locked; holds=editor.selectedLayers; checks=not all locked; writes=comp.layers order by selected runs; source=layer.rs:arrange",
    ),
    (
        "layer.arrange",
        "path=position; resolves=comp,layers,above layer or index; reads=comp.layers order,layer.locked; holds=editor.selectedLayers; checks=above exists and not moved,not all locked; writes=comp.layers order; source=layer.rs:arrange",
    ),
    (
        "layer.precompose",
        "path=leave; resolves=comp,one layer; reads=layer.name,source,source dimensions/duration,comp nested settings,layer.source/name same-field lost-update; holds=editor.selectedLayers; checks=one source-backed layer; writes=new comp and inner layer,outer.source/name,selection,open comp when requested; source=layer.rs:precompose",
    ),
    (
        "layer.precompose",
        "path=move; resolves=comp,layers; reads=selected full layers,parent/matte links,timing,comp.width,height,duration,nested settings,stack; holds=editor.selectedLayers; checks=nonempty selection,valid mode; writes=new comp with moved layers,timing offset,external links cleared,replacement layer,selection,open comp when requested; source=layer.rs:precompose",
    ),
    (
        "layer.addMask",
        "path=append; resolves=comp,layer; reads=layer.source,source dimensions,mask group membership/count; holds=editor.selectedLayers,prefs.appearance.cycleMaskColors; checks=shape valid,masks group supported; writes=new mask group; source=layer.rs:add_mask",
    ),
    (
        "layer.setMask",
        "path=assign; resolves=comp,layer,mask uid/index/name; reads=mask group membership/name/order,assigned mode/inverted same-field lost-update; holds=editor.selectedLayers; checks=mask exists; writes=mask.mode/inverted when supplied; source=layer.rs:mask_props",
    ),
    (
        "layer.addShapeItem",
        "path=append; resolves=comp,layer,group uid/path; reads=contents/group structure,returned match path; holds=editor.selectedLayers; checks=shape contents exists,group exists,kind supported; writes=new contents item; source=layer.rs:add_shape_item",
    ),
    (
        "layer.setText",
        "path=document; resolves=comp,layer; reads=layer timing,text/sourceText evaluated value/keys/expressions,layer.name for auto-rename; holds=editor.currentTime,selectedLayers,prefs recentFonts when font supplied; checks=text layer,attributes valid; writes=text/sourceText at layer time,auto layer.name,preferences; source=layer.rs:set_text,apply_text_params",
    ),
    (
        "layer.setText",
        "path=insertion-style; resolves=comp,layer; reads=text/sourceText and insertion style,layer timing/name; holds=editor.textEdit.pending and target,currentTime,selectedLayers; checks=empty range,no text,editing same layer,nonempty document; writes=editor.textEdit.pending,text/sourceText at time,preferences; source=layer.rs:set_text",
    ),
    (
        "layer.transform",
        "path=reset; resolves=comp,layers; reads=layer.locked,source dimensions,timing,transform property presence/separation/keys same-field lost-update,comp.width,height; holds=editor.currentTime,selectedLayers; checks=not all locked; writes=transform anchor/position/scale/rotation/rotationX/rotationY/orientation/opacity at time; source=layer.rs:transform_op",
    ),
    (
        "layer.transform",
        "path=center; resolves=comp,layers; reads=layer.locked,timing,position/separated axes/keys,Z,comp.width,height; holds=editor.currentTime,selectedLayers; checks=not all locked; writes=position XY at time preserving Z; source=layer.rs:transform_op,update_position",
    ),
    (
        "layer.transform",
        "path=fit; resolves=comp,layers; reads=layer.locked,source dimensions,timing,transform anchor/scale/position/separation/keys,Z,comp.width,height; holds=editor.currentTime,selectedLayers; checks=not all locked,nonzero source dimensions; writes=anchor XY,scale XY,position XY at time; source=layer.rs:transform_op",
    ),
    (
        "layer.transform",
        "path=flip; resolves=comp,layers; reads=layer.locked,timing,scale evaluated value/keys; holds=editor.currentTime,selectedLayers; checks=not all locked; writes=scale flipped X or Y at time; source=layer.rs:transform_op",
    ),
    (
        "layer.settings",
        "path=camera-light; resolves=comp,layer; reads=layer.source; holds=editor.selectedLayers; checks=camera/light source dispatch; writes=delegated cameraSettings/lightSettings fields; source=layer.rs:layer_settings",
    ),
    (
        "layer.settings",
        "path=solid-shared-copy; resolves=comp,layer; reads=layer.source/name,item full clone,all comps source references for shared count,solid assigned fields same-field lost-update; holds=editor.selectedLayers; checks=solid exists,pixelAspect positive,affectAll=false and shared; writes=copy solid,layer.source/name,solid supplied fields; source=layer.rs:layer_settings",
    ),
    (
        "layer.settings",
        "path=solid-or-null; resolves=comp,layer; reads=layer.source/name,all comps source references,solid assigned fields same-field lost-update; holds=editor.selectedLayers; checks=solid exists when solid,pixelAspect positive; writes=layer.name,source solid name/color/width/height/pixelAspect when supplied; source=layer.rs:layer_settings",
    ),
    (
        "layer.quality",
        "path=assign; resolves=comp,layers; reads=layer.quality same-field lost-update; holds=editor.selectedLayers; checks=valid quality; writes=quality; source=layer_menu.rs:quality",
    ),
    (
        "layer.sampling",
        "path=assign; resolves=comp,layers; reads=layer.sampling same-field lost-update; holds=editor.selectedLayers; checks=valid sampling; writes=sampling; source=layer_menu.rs:sampling",
    ),
    (
        "layer.frameBlending",
        "path=assign; resolves=comp,layers; reads=layer.frameBlend same-field lost-update; holds=editor.selectedLayers; checks=valid mode; writes=frameBlend; source=layer_menu.rs:frame_blending",
    ),
    (
        "layer.hideOtherVideo",
        "path=hide; resolves=comp,layers; reads=unselected layers.source/isAV,video same-field lost-update,layer membership; holds=editor.selectedLayers; checks=comp exists; writes=other AV layers.video=false; source=layer_menu.rs:hide_other_video",
    ),
    (
        "layer.showAllVideo",
        "path=show; resolves=comp; reads=all layer.trackMatte references,nonmatte video same-field lost-update,membership; holds=editor.activeComp fallback; checks=comp exists; writes=nonmatte layers.video=true; source=layer_menu.rs:show_all_video",
    ),
    (
        "layer.unlockAll",
        "path=unlock; resolves=comp; reads=all layer.locked same-field lost-update,membership; holds=editor.activeComp fallback; checks=comp exists; writes=all layer.locked=false; source=layer_menu.rs:unlock_all",
    ),
    (
        "layer.expressions",
        "path=assign; resolves=comp,layers; reads=all target property expression presence/enabled same-field lost-update,result count; holds=editor.selectedLayers; checks=comp exists; writes=existing expression.enabled; source=layer_menu.rs:expressions",
    ),
    (
        "layer.setTransform",
        "path=combined; resolves=comp,layers,transform member; reads=layer timing,property presence/type/value/keys,same-field lost-update,Z for two-vector; holds=editor.currentTime,selectedLayers; checks=member/value valid,coercion; writes=transform property at time; source=layer_menu.rs:set_transform",
    ),
    (
        "layer.setTransform",
        "path=separated-position; resolves=comp,layers; reads=layer timing,positionX/Y/Z values/keys,separation presence,combined position; holds=editor.currentTime,selectedLayers; checks=value coercible; writes=positionX/Y/Z and combined position at time; source=layer_menu.rs:set_transform; layer.rs:update_position",
    ),
    (
        "layer.centerAnchor",
        "path=content-center; resolves=comp,layers; reads=layer.locked,source/content bounds,timing,anchor/scale/rotation evaluated,position values/all keys/separation; holds=editor.currentTime,selectedLayers; checks=not all locked,content bounds available; writes=anchor at time,position values/all keys shifted,result count; source=layer_menu.rs:center_anchor; render:content_bounds",
    ),
    (
        "layer.mask.shape",
        "path=target-masks; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership/kind/path keys,timing; holds=editor.currentTime,selectedLayers,selectedProps; checks=targets nonempty,rect length four; writes=mask.path at time; source=layer_menu.rs:target_masks,edit_masks,mask_shape",
    ),
    (
        "layer.mask.set",
        "path=target-masks; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership,field evaluated value/type/keys,timing; holds=editor.currentTime,selectedLayers,selectedProps; checks=targets nonempty,feather/opacity/expansion field,value coercion; writes=mask field at time; source=layer_menu.rs:target_masks,edit_masks,mask_set",
    ),
    (
        "layer.mask.reset",
        "path=target-masks; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership/kind,path/feather/opacity/expansion values/keys/expressions same-field lost-update,source dimensions; holds=editor.selectedLayers,selectedProps; checks=targets nonempty; writes=mask mode/inverted,path/feather/opacity/expansion defaults,cleared keys/expressions; source=layer_menu.rs:mask_reset,edit_masks",
    ),
    (
        "layer.mask.remove",
        "path=selected-or-first; resolves=comp,layers,mask uid/index/name or selectedProps else first; reads=mask full deleted content,membership/order,selection descendant lookup; holds=editor.selectedLayers,selectedProps; checks=none permits zero removed; writes=remove mask groups,selectedProps; source=layer_menu.rs:mask_remove,remove_masks",
    ),
    (
        "layer.mask.removeAll",
        "path=all; resolves=comp,layers; reads=all mask groups full content/membership; holds=editor.selectedLayers,selectedProps; checks=layer exists; writes=remove all masks,selectedProps; source=layer_menu.rs:mask_remove_all,remove_masks",
    ),
    (
        "layer.mask.mode",
        "path=assign; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership/kind/mode same-field lost-update; holds=editor.selectedLayers,selectedProps; checks=targets nonempty,valid mode; writes=mask.mode; source=layer_menu.rs:mask_mode,edit_masks",
    ),
    (
        "layer.mask.invert",
        "path=assign-or-toggle; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership/kind/inverted same-field lost-update or toggle; holds=editor.selectedLayers,selectedProps; checks=targets nonempty; writes=mask.inverted; source=layer_menu.rs:mask_invert,edit_masks",
    ),
    (
        "layer.mask.lock",
        "path=assign-or-toggle; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership/kind/locked same-field lost-update or toggle; holds=editor.selectedLayers,selectedProps; checks=targets nonempty; writes=mask.locked; source=layer_menu.rs:mask_lock,edit_masks",
    ),
    (
        "layer.mask.unlockAll",
        "path=unlock; resolves=comp,layers; reads=mask membership/kind/locked same-field lost-update; holds=editor.selectedLayers; checks=layer exists; writes=all masks.locked=false; source=layer_menu.rs:mask_unlock_all",
    ),
    (
        "layer.mask.lockOthers",
        "path=lock-complement; resolves=comp,layers,mask uid/index/name or selectedProps else all; reads=mask membership/kind/locked same-field lost-update; holds=editor.selectedLayers,selectedProps; checks=target layer exists; writes=mask.locked by membership in keep; source=layer_menu.rs:mask_lock_others",
    ),
    (
        "layer.addMarker",
        "path=replace-at-time; resolves=comp,layers; reads=markersLocked,layer timing,markers at target time and order/result index; holds=editor.currentTime fallback,selectedLayers; checks=nonempty target,locked markers skipped; writes=replace marker at layer time,sorted marker list; source=layer_menu.rs:add_marker",
    ),
    (
        "layer.markersLock",
        "path=assign-or-toggle; resolves=comp,layers; reads=first markersLocked and all assigned markersLocked same-field lost-update; holds=editor.selectedLayers; checks=comp exists; writes=markersLocked; source=layer_menu.rs:markers_lock",
    ),
    (
        "layer.deleteAllMarkers",
        "path=delete; resolves=comp,layers; reads=markersLocked,markers full deleted content; holds=editor.selectedLayers; checks=locked markers skipped; writes=clear markers; source=layer_menu.rs:delete_all_markers",
    ),
    (
        "layer.trackMatte",
        "path=none; resolves=comp,layers; reads=trackMatte same-field lost-update; holds=editor.selectedLayers; checks=valid op; writes=trackMatte=null; source=layer_menu.rs:track_matte_op",
    ),
    (
        "layer.trackMatte",
        "path=above-below; resolves=comp,layers,neighbor by stack; reads=stack order,current trackMatte.kind,neighbor.video same-field lost-update; holds=editor.selectedLayers; checks=neighbor exists; writes=trackMatte,matte.video=false; source=layer_menu.rs:track_matte_op",
    ),
    (
        "layer.trackMatte",
        "path=kind; resolves=comp,layers,above fallback; reads=stack order,current trackMatte reference/kind,matte.video same-field lost-update; holds=editor.selectedLayers; checks=current matte or above exists; writes=trackMatte.kind/reference,matte.video=false; source=layer_menu.rs:track_matte_op",
    ),
    (
        "layer.openSource",
        "path=comp; resolves=comp,layer,layer.source.item; reads=source item kind; holds=editor.selectedLayers; checks=source item exists; writes=editor.openComps/activeComp/selection via open_comp; source=layer_menu.rs:open_source",
    ),
    (
        "layer.openSource",
        "path=noncomp; resolves=comp,layer,layer.source.item; reads=source item kind; holds=editor.selectedLayers; checks=source-backed layer; writes=frontend layer panel event; source=layer_menu.rs:open_source",
    ),
    (
        "layer.revealInFinder",
        "path=file; resolves=comp,layer,source item; reads=layer.source,item.kind/path; holds=editor.selectedLayers; checks=footage path nonempty; writes=open URL event; source=layer_menu.rs:reveal_in_finder",
    ),
    (
        "layer.revealSource",
        "path=reveal; resolves=comp,layer,source item; reads=layer.source; holds=editor.selectedLayers; checks=source-backed layer; writes=editor.projectSelection,frontend project-panel event; source=layer_menu.rs:reveal_source",
    ),
    (
        "comp.revealInProject",
        "path=reveal; resolves=comp; reads=comp existence; holds=editor.activeComp fallback; checks=comp exists; writes=editor.projectSelection,frontend project-panel event; source=layer_menu.rs:reveal_comp",
    ),
    (
        "layer.revealExpressionErrors",
        "path=evaluate; resolves=comp; reads=all layer timing,expression properties/evaluated dependencies; holds=editor.compTime,expression host; checks=comp exists; writes=frontend reveal properties event,toast when no errors; source=layer_menu.rs:reveal_expression_errors,expression_errors",
    ),
    (
        "layer.sequence",
        "path=timing; resolves=comp,layers in supplied or selection order; reads=layer.startTime,inPoint,outPoint,stretch; holds=editor.selectedLayers order; checks=at least two layers; writes=following layers timing; source=layer_menu.rs:sequence",
    ),
    (
        "layer.sequence",
        "path=transition; resolves=comp,layers in supplied or selection order; reads=layer.startTime,inPoint,outPoint,stretch,opacity property/keys same-field lost-update; holds=editor.selectedLayers order; checks=at least two layers,overlap and transition nonoff; writes=timing,opacity keys replacing old keys; source=layer_menu.rs:sequence",
    ),
    (
        "layer.timeStretch",
        "path=stretch; resolves=comp,layers; reads=layer.stretch,startTime,inPoint,outPoint,comp.frameRate; holds=editor.currentTime when hold=current,selectedLayers; checks=abs stretch >=1; writes=stretch,startTime,inPoint,outPoint; source=layer_time.rs:time_stretch",
    ),
    (
        "layer.timeStretch",
        "path=reverse; resolves=comp,layers; reads=layer.stretch,startTime,inPoint,outPoint,comp.frameRate; holds=editor.selectedLayers; checks=comp exists; writes=stretch,startTime; source=layer_time.rs:time_stretch",
    ),
    (
        "layer.timeReverse",
        "path=reverse; resolves=comp,layers; reads=layer.stretch,startTime,inPoint,outPoint,comp.frameRate; holds=editor.selectedLayers; checks=comp exists; writes=stretch,startTime; source=layer_time.rs:time_stretch",
    ),
    (
        "layer.enableTimeRemap",
        "path=enable; resolves=comp,layers; reads=layer.source,timing,timeRemap presence,source duration,comp.frameRate; holds=editor.selectedLayers; checks=some footage/precomp source,per-layer remappable; writes=new timeRemap property and endpoint keys only when missing; source=layer_time.rs:enable_time_remap",
    ),
    (
        "layer.enableTimeRemap",
        "path=disable; resolves=comp,layers; reads=layer.source,timeRemap full deleted property; holds=editor.selectedLayers; checks=some footage/precomp source; writes=remove timeRemap; source=layer_time.rs:enable_time_remap",
    ),
    (
        "layer.freezeFrame",
        "path=freeze; resolves=comp,layers; reads=layer.source,timing,timeRemap evaluated value/keys/expressions,source-time dependencies,comp.frameRate; holds=editor.currentTime,selectedLayers,expression/footage host; checks=some remappable source; writes=timeRemap value and held key replacing keys; source=layer_time.rs:freeze; EvalCtx::source_time",
    ),
    (
        "layer.freezeOnLastFrame",
        "path=freeze-last; resolves=comp,layers; reads=layer.source,timing,timeRemap evaluated value/keys/expressions,source-time dependencies,comp.frameRate,duration; holds=editor.selectedLayers,expression/footage host; checks=some remappable source; writes=timeRemap value/two keys replacing keys,outPoint=comp duration; source=layer_time.rs:freeze; EvalCtx::source_time",
    ),
];

#[test]
fn independent_layer_rows_name_registered_commands_and_unique_paths() {
    let specs = kition_motion_engine::commands::command_specs();
    let mut paths = std::collections::BTreeSet::new();
    for (command, row) in ROWS {
        assert!(specs.iter().any(|spec| spec.id == *command), "unregistered command {command}");
        let path = row.split(';').next().and_then(|field| field.strip_prefix("path=")).expect("path field");
        assert!(paths.insert((*command, path)), "duplicate path {command}/{path}");
        for field in ["resolves=", "reads=", "holds=", "checks=", "writes=", "source="] {
            assert!(row.split("; ").any(|part| part.starts_with(field)), "missing {field} in {command}/{path}");
        }
    }
}

// Pinned after declaration review. These structured expectations come from the readers cited
// above, including shared resolvers and delegated commands; they are not blind observations.
// Reads: p=active composition, s=layer/property/key/vertex selection, t=current time. Object
// roles name parameters consumed from the caller, excluding the primary layer target. Project
// selection, queue ids, effect/property uids, import-file layer selectors and synthesized inner
// parameters are not object roles here. Persisted field/path premises remain prose in ROWS.
pub(crate) const DECLARATIONS: &[(&str, &str, &str, &str)] = &[
    ("renderQueue.add", "p", "", "comp:Item"),
    ("renderQueue.remove", "", "", ""),
    ("renderQueue.setRender", "", "", ""),
    ("renderQueue.setRenderSettings", "", "", ""),
    ("renderQueue.setOutputModule", "", "", ""),
    ("renderQueue.setOutput", "", "", ""),
    ("render.addOutputModule", "", "", ""),
    ("render.preRender", "p", "", "comp:Item"),
    ("render.saveCurrentPreview", "p", "", "comp:Item"),
    ("renderQueue.move", "", "", ""),
    ("renderQueue.duplicate", "", "", ""),
    ("renderQueue.render", "pst", "", ""),
    ("renderQueue.stop", "", "", ""),
    ("renderQueue.list", "", "", ""),
    ("renderQueue.saveTemplate", "", "", ""),
    ("renderQueue.deleteTemplate", "", "", ""),
    ("renderQueue.setTemplateDefault", "", "", ""),
    ("renderQueue.applyTemplate", "", "", ""),
    ("renderQueue.setLog", "", "", ""),
    ("renderQueue.setNotify", "", "", ""),
    ("renderQueue.setOverflowFolders", "", "", ""),
    ("file.newProject", "p", "", ""),
    ("file.openDemoProject", "p", "", ""),
    ("file.open", "pst", "", ""),
    ("file.save", "", "", ""),
    ("file.saveAs", "", "", ""),
    ("file.incrementAndSave", "", "", ""),
    ("file.revert", "pst", "", ""),
    ("file.import", "pst", "", ""),
    ("file.projectSettings", "", "", ""),
    ("file.cycleBitDepth", "", "", ""),
    ("render.backend", "", "", ""),
    ("project.newFolder", "", "", "parent:Item"),
    ("file.close", "p", "", "comp:Item"),
    ("file.closeProject", "p", "", ""),
    ("file.saveCopy", "", "", ""),
    ("file.importMultiple", "pst", "", ""),
    ("file.importPlaceholder", "", "", ""),
    ("file.importSolid", "p", "", ""),
    // Source-supported correction for maintainer review: file_more.rs:218 opens a fresh comp before layer.addItem;
    // Session::open_comp (lib.rs:895-902) clears the caller's layer selection first.
    ("file.newCompFromSelection", "pt", "", ""),
    ("file.removeUnusedFootage", "pt", "", ""),
    ("file.consolidateFootage", "pt", "", ""),
    ("file.reduceProject", "pt", "", ""),
    ("file.findMissing", "", "", ""),
    ("file.collectFiles", "", "", ""),
    ("file.runScript", "pst", "", ""),
    ("script.run", "pst", "", ""),
    ("file.interpretFootage", "ps", "", "item:Item items:Item"),
    ("file.rememberInterpretation", "ps", "", "item:Item items:Item"),
    ("file.applyInterpretation", "ps", "", "item:Item items:Item"),
    ("file.replaceFootage", "ps", "", "item:Item items:Item"),
    ("file.replaceWithPlaceholder", "ps", "", "item:Item items:Item"),
    ("file.replaceWithSolid", "ps", "", "item:Item items:Item"),
    ("file.reloadFootage", "ps", "", "item:Item items:Item"),
    ("footage.check", "pst", "", "items:Item"),
    ("footage.reloadChanged", "pst", "", ""),
    ("file.revealInFinder", "ps", "", "item:Item items:Item"),
    ("file.relinkFootage", "pst", "", ""),
    ("file.setProxy", "", "", "item:Item items:Item"),
    ("file.setProxyNone", "", "", "item:Item items:Item"),
    ("file.useProxy", "", "", "item:Item items:Item"),
    ("file.interpretProxy", "", "", "item:Item items:Item"),
    ("file.createProxy", "pt", "", "comp:Item item:Item"),
    ("layer.sceneEditDetection", "pst", "layer", "comp:Item"),
    ("file.exportLottie", "p", "", "comp:Item"),
    ("file.importLottie", "p", "", ""),
    ("effect.apply", "ps", "layers", "comp:Item layer:Layer"),
    ("effect.applyLast", "ps", "layers", "comp:Item layer:Layer"),
    ("effect.pickColor", "pst", "layer", "comp:Item"),
    ("effect.remove", "ps", "layer", "comp:Item"),
    ("effect.removeAll", "ps", "layers", "comp:Item layer:Layer"),
    ("effect.toggle", "ps", "layer", "comp:Item"),
    ("effect.reorder", "ps", "layer", "comp:Item"),
    ("effect.duplicate", "ps", "layer", "comp:Item"),
    ("effect.copy", "ps", "layer", "comp:Item"),
    ("effect.paste", "ps", "layers", "comp:Item layer:Layer"),
    ("effect.reset", "pst", "layer", "comp:Item"),
    ("effect.plugins.load", "", "", ""),
    ("edit.undo", "pt", "", ""),
    ("edit.redo", "pt", "", ""),
    ("edit.duplicate", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.clear", "pst", "layers", "comp:Item layer:Layer"),
    ("edit.copy", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.copyWithPropertyLinks", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.copyWithRelativePropertyLinks", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.copyExpressionOnly", "ps", "", ""),
    ("edit.cut", "pst", "layers", "comp:Item layer:Layer"),
    ("edit.paste", "pst", "", "comp:Item layer:Layer layers:Layer"),
    ("edit.pasteReversedKeyframes", "pst", "layers", "comp:Item layer:Layer"),
    ("edit.splitLayer", "pst", "layers", "comp:Item layer:Layer"),
    ("edit.liftWorkArea", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.extractWorkArea", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.selectAll", "ps", "", ""),
    ("edit.deselectAll", "", "", ""),
    ("edit.label", "ps", "layers", "comp:Item layer:Layer"),
    ("edit.selectLabelGroup", "ps", "", ""),
    ("edit.purgeUndo", "", "", ""),
    ("edit.purge", "", "", ""),
    ("edit.editOriginal", "ps", "", ""),
    ("edit.history.goto", "pt", "", ""),
    ("comp.new", "p", "", ""),
    ("comp.settings", "p", "", "comp:Item"),
    ("comp.open", "p", "", "comp:Item item:Item"),
    ("comp.close", "p", "", "comp:Item"),
    ("comp.workArea", "pt", "", "comp:Item"),
    ("comp.setSwitch", "p", "", "comp:Item"),
    ("comp.addMarker", "pt", "", "comp:Item"),
    ("comp.setPosterTime", "pt", "", "comp:Item"),
    ("comp.trimToWorkArea", "p", "", "comp:Item"),
    ("comp.cropToRegionOfInterest", "p", "", "comp:Item"),
    ("comp.cropToLayerBounds", "pst", "layers", "comp:Item layer:Layer"),
    ("comp.saveFrameAs", "pt", "", "comp:Item"),
    ("comp.responsiveTime", "pt", "", "comp:Item"),
    ("layer.create", "pst", "layers", "comp:Item layer:Layer"),
    ("project.select", "", "", "items:Item"),
    ("project.rename", "", "", "item:Item"),
    ("project.move", "", "", "folder:Item item:Item items:Item"),
    ("project.setLabel", "", "", "item:Item items:Item"),
    ("project.setComment", "", "", "item:Item items:Item"),
    ("project.delete", "pt", "", "item:Item items:Item"),
    ("project.duplicate", "", "", "item:Item items:Item"),
    ("engine.batch", "pst", "", "comp:Item"),
    ("layer.newSolid", "ps", "", "comp:Item"),
    ("layer.newAdjustment", "ps", "", "comp:Item"),
    ("layer.newNull", "ps", "", "comp:Item"),
    ("layer.newText", "ps", "", "comp:Item"),
    ("layer.newShape", "ps", "", "comp:Item"),
    ("layer.addItem", "pst", "", "comp:Item item:Item"),
    ("layer.select", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.selectNext", "ps", "", ""),
    ("layer.selectPrevious", "ps", "", ""),
    ("layer.setSwitch", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.rename", "ps", "layer", "comp:Item"),
    ("layer.setComment", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.setBlendMode", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.setTrackMatte", "ps", "layer", "comp:Item matte:Layer"),
    ("layer.setParent", "pst", "layers", "comp:Item layer:Layer parent:Layer"),
    ("layer.timing", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.slip", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.slipBack", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.slipForward", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.arrange", "ps", "layers", "above:Layer comp:Item layer:Layer"),
    ("layer.precompose", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.addMask", "ps", "layer", "comp:Item"),
    ("layer.setMask", "ps", "layer", "comp:Item"),
    ("layer.addShapeItem", "ps", "layer", "comp:Item"),
    ("layer.setText", "pst", "layer", "comp:Item"),
    ("layer.transform", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.settings", "pst", "layer", "comp:Item"),
    ("layer.quality", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.sampling", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.frameBlending", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.hideOtherVideo", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.showAllVideo", "p", "", "comp:Item"),
    ("layer.unlockAll", "p", "", "comp:Item"),
    ("layer.expressions", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.setTransform", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.centerAnchor", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.mask.shape", "pst", "layer", "comp:Item layers:Layer"),
    ("layer.mask.set", "pst", "layer", "comp:Item layers:Layer"),
    ("layer.mask.reset", "pst", "layer", "comp:Item layers:Layer"),
    ("layer.mask.remove", "ps", "layer", "comp:Item layers:Layer"),
    ("layer.mask.removeAll", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.mask.mode", "pst", "layer", "comp:Item layers:Layer"),
    ("layer.mask.invert", "pst", "layer", "comp:Item layers:Layer"),
    ("layer.mask.lock", "pst", "layer", "comp:Item layers:Layer"),
    ("layer.mask.unlockAll", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.mask.lockOthers", "ps", "layer", "comp:Item layers:Layer"),
    ("layer.addMarker", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.markersLock", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.deleteAllMarkers", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.trackMatte", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.openSource", "ps", "layer", "comp:Item"),
    ("layer.revealInFinder", "ps", "layer", "comp:Item"),
    ("layer.revealSource", "ps", "layer", "comp:Item"),
    ("comp.revealInProject", "p", "", "comp:Item"),
    ("layer.revealExpressionErrors", "pt", "", "comp:Item"),
    ("layer.sequence", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.timeStretch", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.timeReverse", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.enableTimeRemap", "ps", "layers", "comp:Item layer:Layer"),
    ("layer.freezeFrame", "pst", "layers", "comp:Item layer:Layer"),
    ("layer.freezeOnLastFrame", "pst", "layers", "comp:Item layer:Layer"),
];

// These SDK states were reviewed after the PST table was pinned. A text-edit session chooses
// both a command's editing mode and its active object. An ROI is the selected canvas area.
// Project-panel selection and clipboard contents have no exact mapping to Motion's call context:
// `on` names comp layers, while SDK Active describes one active destination, not a project list.
pub(crate) const ADDITIONAL_READS: &[(&str, &[&str])] = &[
    ("comp.cropToRegionOfInterest", &["area"]),
    ("renderQueue.add", &["area"]),
    ("renderQueue.setOutputModule", &["area"]),
    ("renderQueue.saveTemplate", &["area"]),
    ("render.addOutputModule", &["area"]),
    ("render.preRender", &["area"]),
    ("edit.clear", &["active", "mode"]),
    ("edit.copy", &["active", "mode"]),
    ("edit.cut", &["active", "mode"]),
    ("edit.paste", &["active", "mode"]),
    ("edit.selectAll", &["active", "mode"]),
    ("layer.setText", &["active", "mode"]),
];

fn scalar_reference_schema(node: &serde_json::Value) -> bool {
    match node.get("oneOf").and_then(serde_json::Value::as_array) {
        Some(branches) => !branches.is_empty() && branches.iter().all(scalar_reference_schema),
        None => matches!(node.get("type").and_then(serde_json::Value::as_str), Some("integer" | "string")),
    }
}

#[test]
fn source_layer_declarations_match_catalogue() {
    use std::collections::{BTreeMap, BTreeSet};

    use kition_kit::KitDocument;

    let doc = kition_motion_kit::MotionDocument::create("LayerRead.kmotion", 64, 36).expect("fixture");
    let commands: BTreeMap<_, _> = doc.commands().into_iter().map(|command| (command.id.clone(), command)).collect();
    let row_ids: BTreeSet<_> = ROWS.iter().map(|(id, _)| *id).collect();
    let expected_ids: BTreeSet<_> = DECLARATIONS.iter().map(|(id, _, _, _)| *id).collect();
    assert_eq!(DECLARATIONS.len(), expected_ids.len(), "duplicate structured command reading");
    assert_eq!(row_ids, expected_ids, "structured readings must cover every source-read command");
    let mut compared = 0;
    let mut different = BTreeSet::new();
    let mut differences = BTreeMap::<&str, Vec<String>>::new();
    for (id, reads, target, roles) in DECLARATIONS {
        let actual = commands.get(*id).expect("registered command");
        if !actual.mutates {
            continue;
        }
        compared += 1;
        let mut expected_reads: BTreeSet<_> = reads
            .chars()
            .map(|read| match read {
                'p' => "part",
                's' => "selection",
                't' => "time",
                other => panic!("unknown read {other}"),
            })
            .collect();
        expected_reads.extend(ADDITIONAL_READS.iter().filter(|(command, _)| command == id).flat_map(|(_, reads)| reads.iter().copied()));
        let actual_reads: BTreeSet<_> = actual.reads.iter().map(|read| read.as_str()).collect();
        let expected_roles: BTreeMap<_, _> = roles
            .split_whitespace()
            .map(|role| role.split_once(':').expect("parameter:type"))
            // Motion rejects unknown top-level parameters before dispatch. Keep unreachable
            // handler inputs in DECLARATIONS and report them separately, not as guard gaps.
            .filter(|(parameter, _)| actual.schema.as_ref().and_then(|schema| schema.get(parameter.split('.').next().unwrap_or(parameter))).is_some())
            .collect();
        let expected_objects: BTreeSet<_> = expected_roles.keys().copied().collect();
        let actual_objects: BTreeSet<_> = actual.objects.iter().copied().collect();
        let mut expected_types: BTreeSet<String> = expected_roles.iter().map(|(parameter, kind)| format!("{parameter}:{kind}")).collect();
        if !target.is_empty() {
            expected_types.insert(format!("{target}:Layer"));
        }
        let actual_types: BTreeSet<String> = actual
            .object_types
            .iter()
            .flat_map(|(parameter, kinds)| kinds.iter().map(move |kind| (*parameter, *kind)))
            .filter(|(parameter, kind)| {
                let Some(parent) = parameter.strip_suffix("[]") else { return true };
                let array = actual.schema.as_ref().and_then(|schema| schema.get(parent));
                // Per-item types duplicate a list's logical role so its name marks stay typed.
                // Collapse only the identical type on a declared list of scalar references.
                !(actual.objects.contains(&parent)
                    && array.is_some_and(|node| node.get("type").and_then(serde_json::Value::as_str) == Some("array"))
                    && array.and_then(|node| node.get("items")).is_some_and(scalar_reference_schema)
                    && actual.object_types.iter().any(|(name, kinds)| *name == parent && kinds.contains(kind)))
            })
            .map(|(parameter, kind)| format!("{parameter}:{kind}"))
            .collect();
        let mut add = |class, missing: Vec<String>, extra: Vec<String>| {
            if !missing.is_empty() || !extra.is_empty() {
                different.insert(*id);
                differences.entry(class).or_default().push(format!("{id}: less={missing:?}; more={extra:?}"));
            }
        };
        add(
            "reads",
            expected_reads.difference(&actual_reads).map(|value| (*value).to_owned()).collect(),
            actual_reads.difference(&expected_reads).map(|value| (*value).to_owned()).collect(),
        );
        let expected_target = (!target.is_empty()).then_some(*target);
        if actual.targets != expected_target {
            add("targets", expected_target.into_iter().map(str::to_owned).collect(), actual.targets.into_iter().map(str::to_owned).collect());
        }
        add(
            "objects",
            expected_objects.difference(&actual_objects).map(|value| (*value).to_owned()).collect(),
            actual_objects.difference(&expected_objects).map(|value| (*value).to_owned()).collect(),
        );
        add("object_types", expected_types.difference(&actual_types).cloned().collect(), actual_types.difference(&expected_types).cloned().collect());
    }
    println!("LAYER_DECLARATIONS compared={compared} differing={}", different.len());
    let mut report = Vec::new();
    for (class, rows) in &differences {
        report.push(format!("{class}: {} commands\n{}", rows.len(), rows.join("\n")));
    }
    assert!(differences.is_empty(), "{} differing commands out of {compared}\n{}", different.len(), report.join("\n"));
}

#[test]
fn project_item_lists_declare_per_item_names_and_types() {
    use kition_kit::KitDocument;

    let doc = kition_motion_kit::MotionDocument::create("ItemArrays.kmotion", 64, 36).expect("fixture");
    let commands = doc.commands();
    // project_items.rs:item_ref/items_p reads every list member as an item id or item name.
    for id in ["project.select", "project.move", "project.setLabel", "project.setComment", "project.delete", "project.usage", "project.duplicate"] {
        let command = commands.iter().find(|command| command.id == id).expect("registered command");
        let array = command.schema.as_ref().and_then(|schema| schema.get("items")).expect("item list schema");
        assert_eq!(array["type"], "array", "{id}");
        assert!(array.get("items").is_some_and(scalar_reference_schema), "{id}: each member is a scalar item reference: {array}");
        assert!(command.objects.contains(&"items"), "{id}: the list remains the logical object parameter");
        assert!(command.name_params.contains(&("items[]", "name")), "{id}: names belong to list members");
        assert!(!command.name_params.contains(&("items", "name")), "{id}: a list-level name mark hides numeric references");
        for path in ["items", "items[]"] {
            assert!(command.object_types.iter().any(|(name, kinds)| *name == path && *kinds == ["Item"]), "{id}: {path} is typed as Item");
        }
    }
}

#[test]
fn source_layer_role_schema_reachability() {
    use std::collections::{BTreeMap, BTreeSet};

    use kition_kit::KitDocument;

    let doc = kition_motion_kit::MotionDocument::create("LayerRoles.kmotion", 64, 36).expect("fixture");
    let commands: BTreeMap<_, _> = doc.commands().into_iter().map(|command| (command.id.clone(), command)).collect();
    let mut absent = Vec::new();
    let mut accepted = Vec::new();
    let mut absent_ids = BTreeSet::new();
    let mut accepted_ids = BTreeSet::new();
    for (id, _, _, roles) in DECLARATIONS {
        let actual = commands.get(*id).expect("registered command");
        if !actual.mutates {
            continue;
        }
        let actual_objects: BTreeSet<_> = actual.objects.iter().copied().collect();
        for (parameter, kind) in roles.split_whitespace().map(|role| role.split_once(':').expect("parameter:type")) {
            if actual_objects.contains(parameter) {
                continue;
            }
            if actual.schema.as_ref().and_then(|schema| schema.get(parameter)).is_some() {
                accepted_ids.insert(*id);
                accepted.push(format!("{id}.{parameter}:{kind}"));
            } else {
                absent_ids.insert(*id);
                absent.push(format!("{id}.{parameter}:{kind}"));
            }
        }
    }
    println!("LAYER_ROLES schema_absent={} parameters/{} commands: {}", absent.len(), absent_ids.len(), absent.join(", "));
    println!("LAYER_ROLES schema_accepted={} parameters/{} commands: {}", accepted.len(), accepted_ids.len(), accepted.join(", "));
}

struct FieldReading {
    case: &'static str,
    command: &'static str,
    params: serde_json::Value,
    premises: Vec<kition_kit::Premise>,
    source: &'static str,
}

fn field_premise(id: u64, fields: &[&str], positions: &[&str]) -> kition_kit::Premise {
    kition_kit::Premise::on(serde_json::json!(id)).fields(fields.iter().copied()).positions(positions.iter().copied())
}

// Pinned after declaration exposure. These are explicit-id scenarios, not a complete reading of
// the family. Windows, list membership, name/index resolution and dynamic expressions are not
// compared here. Every field below is read or replaced in the cited scenario.
fn source_field_readings() -> (kition_motion_kit::MotionDocument, Vec<FieldReading>) {
    use kition_kit::{CallContext, KitDocument, Premise};
    use kition_motion_engine::Session;
    use kition_motion_project::{ItemId, LayerId};
    use serde_json::json;

    let mut session = Session::default();
    let main = session.execute("comp.new", json!({"name":"Main","width":320,"height":180,"frameRate":30,"duration":10})).expect("main")["comp"]
        .as_u64()
        .expect("comp id");
    let solid = session.execute("layer.newSolid", json!({"name":"Solid"})).expect("solid")["layer"].as_u64().expect("layer id");
    let solid_item = session.project.comp(ItemId(main)).and_then(|comp| comp.layer(LayerId(solid))).and_then(|layer| layer.source.item()).expect("source").0;
    session.execute("layer.addItem", json!({"item":solid_item})).expect("shared solid");
    let parent = session.execute("layer.newNull", json!({"name":"Parent"})).expect("parent")["layer"].as_u64().expect("layer id");
    let ancestor = session.execute("layer.newNull", json!({"name":"Ancestor"})).expect("ancestor")["layer"].as_u64().expect("layer id");
    session.execute("layer.setParent", json!({"layers":[parent],"parent":ancestor,"compensate":false})).expect("parent chain");
    let footage =
        session.execute("file.importPlaceholder", json!({"name":"Offline","width":640,"height":360,"frameRate":24,"duration":6})).expect("footage")["item"]
            .as_u64()
            .expect("item id");
    let footage_layer = session.execute("layer.addItem", json!({"item":footage})).expect("footage layer")["layer"].as_u64().expect("layer id");
    let empty = session.execute("comp.new", json!({"name":"Empty","width":320,"height":180,"frameRate":30,"duration":10,"open":false})).expect("empty")["comp"]
        .as_u64()
        .expect("comp id");
    let layers: Vec<_> = session.project.comp(ItemId(main)).expect("main comp").layers.iter().map(|layer| layer.id.0).collect();
    let items: Vec<_> = session.project.items.iter().map(|(id, item)| (id.0, item.as_comp().is_some())).collect();
    let effects_group =
        session.project.comp(ItemId(main)).and_then(|comp| comp.layer(LayerId(solid))).and_then(|layer| layer.props.sub("effects")).expect("effects").uid;
    let mut doc = kition_motion_kit::MotionDocument::open("FieldRead.kmotion", session.project.to_json().as_bytes()).expect("fixture");
    doc.set_context(&CallContext { part: Some(json!({"comp":main})), time: Some(1.0), on: vec![json!(solid)] }).expect("context");

    // Updated at the kition-sdk re-pin (maintainer-approved). The shared-solid scan reads every
    // item's kind and holds the target layer's stack place; the copied solid holds the fields
    // the clone copies, not its comment (C8-effect-read acc81a36).
    let source_users = || layers.iter().map(|id| field_premise(*id, &["attrs.source", "sourceItem"], &[])).collect::<Vec<_>>();
    let mut settings: Vec<_> = source_users().into_iter().filter(|premise| premise.id != json!(solid)).collect();
    settings.push(Premise::on(json!(solid)).fields(["attrs.source", "sourceItem"]));
    for (id, _) in &items {
        let fields: &[&str] = if *id == solid_item { &["kind", "data", "attrs.label", "parent", "name"] } else { &["kind"] };
        settings.push(field_premise(*id, fields, &[]));
    }
    // Retargeting scans every item's kind and every composition's layer list (C8-motion-pin
    // add8bd6d). The comp size is read only for a source without dimensions (C8-motion-pin
    // 60219209). The replaced item's `data` is read for its size (and a placeholder's rate and
    // duration), so its premise names `data` (C8-motion-pin, sdk_replace_item_data).
    let retargeted = || {
        let mut premises = source_users();
        for (id, comp) in &items {
            let fields: &[&str] = if *id == footage {
                &["kind", "data"]
            } else if *comp {
                &["kind", "~layerMembers"]
            } else {
                &["kind"]
            };
            premises.push(field_premise(*id, fields, &[]));
        }
        premises
    };
    let replace_solid = retargeted();
    let replace_placeholder = retargeted();

    let rows = vec![
        FieldReading {
            case: "comp-scalar-background",
            command: "comp.settings",
            params: json!({"comp":empty,"background":"#112233"}),
            // Only the assigned fields and the renderer branch (C8-motion-read cf1322ae).
            premises: vec![field_premise(empty, &["kind", "comp.attrs.renderer", "comp.attrs.background"], &[])],
            source: "commands/comp.rs:27 apply_settings,136 settings; commands/model3d.rs:30 sync_geometry_options (classic renderer returns before layers)",
        },
        FieldReading {
            case: "work-area-clamps",
            command: "comp.workArea",
            params: json!({"comp":main,"start":1}),
            premises: vec![field_premise(main, &["comp.attrs.work_area", "comp.attrs.frame_rate", "comp.attrs.duration"], &[])],
            source: "commands/comp.rs:170 work_area,201 keep_work_area_ordered",
        },
        FieldReading {
            case: "shared-solid-settings",
            command: "layer.settings",
            params: json!({"layer":solid,"color":"#abcdef","affectAll":false}),
            premises: settings,
            source: "commands/layer.rs:1394 layer_settings (source scan and cloned solid item)",
        },
        FieldReading {
            case: "uncompensated-parent-chain",
            command: "layer.setParent",
            params: json!({"layers":[solid],"parent":parent,"compensate":false}),
            // The new parent is held whole (more than its parent field).
            premises: vec![field_premise(solid, &["parent"], &[]), Premise::on(json!(parent)), field_premise(ancestor, &["parent"], &[])],
            source: "commands/layer.rs:798 check_parent_chain,821 set_parent; compensation disabled",
        },
        FieldReading {
            case: "leave-precompose-defaults",
            command: "layer.precompose",
            params: json!({"layers":[solid],"mode":"leave"}),
            premises: vec![
                // The precomposed layer is held whole (C8-motion-pin add8bd6d).
                Premise::on(json!(solid)),
                field_premise(solid_item, &["kind", "data.width", "data.height", "attrs.label"], &[]),
                field_premise(
                    main,
                    &[
                        "comp.attrs.duration",
                        "comp.attrs.frame_rate",
                        "comp.attrs.pixel_aspect",
                        "comp.attrs.background",
                        "comp.attrs.shutter_angle",
                        "comp.attrs.shutter_phase",
                        "comp.attrs.motion_blur_samples",
                        "comp.attrs.motion_blur_adaptive_limit",
                        "comp.attrs.renderer",
                        "comp.attrs.enable_motion_blur",
                        "comp.attrs.enable_frame_blending",
                        "comp.attrs.preserve_frame_rate",
                        "comp.attrs.preserve_resolution",
                    ],
                    &[],
                ),
            ],
            source: "commands/layer.rs:1018 precompose leave; project/lib.rs:411 nested_like; project/build.rs:721 default_label,737 layer; render/eval.rs:328 source_size",
        },
        FieldReading {
            case: "effect-source-dimension-defaults",
            command: "effect.apply",
            params: json!({"layers":[solid],"effect":"Gradient Ramp"}),
            premises: vec![
                // The effects group by uid, not the whole list (C8-effect-read 91336e1e).
                field_premise(
                    solid,
                    &[
                        "attrs.source",
                        "sourceItem",
                        &format!("props.children[uid={effects_group}].uid"),
                        &format!("props.children[uid={effects_group}].node"),
                        &format!("props.children[uid={effects_group}].meta.match"),
                    ],
                    &[],
                ),
                field_premise(solid_item, &["kind", "data.width", "data.height"], &[]),
            ],
            source: "commands/effect.rs:153 apply; render/eval.rs:317 effect_bounds,328 source_size; effects instantiate scales point defaults with size",
        },
        FieldReading {
            case: "replace-solid-users",
            command: "file.replaceWithSolid",
            params: json!({"item":footage,"color":"#445566"}),
            premises: replace_solid,
            source: "commands/file_more.rs:126 solid_from,862 retarget_layers,898 replace_with_solid",
        },
        FieldReading {
            case: "replace-placeholder-users",
            command: "file.replaceWithPlaceholder",
            params: json!({"item":footage}),
            premises: replace_placeholder,
            source: "commands/file_more.rs:862 retarget_layers,876 replace_with_placeholder",
        },
        FieldReading {
            case: "slip-source-span",
            command: "layer.slip",
            params: json!({"layers":[footage_layer],"delta":1}),
            premises: vec![
                // Remap presence only (C8-effect-read e613f506).
                field_premise(footage_layer, &["attrs.source", "sourceItem", "attrs.stretch", "timeRemap.meta.match"], &["startTime", "inPoint", "outPoint"]),
                field_premise(footage, &["kind", "data.kind", "data.duration", "data.loop_count"], &[]),
                field_premise(main, &["comp.attrs.frame_rate"], &[]),
            ],
            source: "commands/layer.rs:902 slip (no remap, video placeholder)",
        },
        FieldReading {
            case: "remap-source-endpoint",
            command: "layer.enableTimeRemap",
            params: json!({"layers":[footage_layer],"value":true}),
            premises: vec![
                // Remap presence only; the frame rate only for a reversed layer (C8-effect-read e613f506).
                field_premise(footage_layer, &["attrs.source", "sourceItem", "attrs.stretch", "timeRemap.meta.match"], &["startTime", "inPoint", "outPoint"]),
                field_premise(footage, &["kind", "data.kind", "data.duration"], &[]),
            ],
            source: "commands/layer_time.rs:86 enable_time_remap (source duration clamps endpoint)",
        },
    ];
    (doc, rows)
}

// Effective explicit-id needs, following pinned SDK 34dff06 kition-kits/src/tools/guard.rs:
// 352-379 params records target ids and typed mentioned object params; 1698-1702 asks under for
// every step; 1729-1759 inserts explicit premises over that step's target defaults; 1767-1801
// adds resolved mentioned objects with Need::all only if a premise did not already name them.
// Target independence is Need::of_command at 75-76; objects use Need::all. These cases use numeric
// ids of the right type, no names/indexes and no at.time place. Under.objects is not added unless
// the SDK treats a step as placed (1740-1749); here it is empty even for the work-area time case.
fn effective_explicit_needs(
    doc: &kition_motion_kit::MotionDocument,
    command: &str,
    params: &serde_json::Value,
) -> std::collections::BTreeMap<String, kition_kit::Premise> {
    use kition_kit::{KitDocument, Premise};

    let declared = doc.commands().into_iter().find(|found| found.id == command).expect("registered command");
    let mut needs = std::collections::BTreeMap::new();
    for parameter in declared.targets.into_iter().chain(declared.objects.iter().copied()) {
        let value = params.get(parameter);
        let values: Vec<_> = match value {
            Some(serde_json::Value::Array(values)) => values.iter().collect(),
            Some(value) => vec![value],
            None => Vec::new(),
        };
        for value in values {
            let id = value.as_u64().expect("explicit numeric object id");
            let mut premise = Premise::on(serde_json::json!(id));
            if declared.targets == Some(parameter) && declared.position_independent {
                premise = premise.positions(Vec::<String>::new());
            }
            needs.insert(id.to_string(), premise);
        }
    }
    let under = doc.under(command, params).expect("under").unwrap_or_default();
    assert!(under.objects.is_empty(), "{command}: unmodelled place objects {:?}", under.objects);
    for premise in under.premises {
        needs.insert(premise.id.as_u64().expect("numeric premise object").to_string(), premise);
    }
    needs
}

#[test]
fn source_layer_field_premises_match_explicit_ids() {
    use std::collections::{BTreeMap, BTreeSet};

    let (doc, cases) = source_field_readings();
    let mut differences = Vec::new();
    let mut less_fields = 0;
    let mut more_fields = 0;
    let mut less_positions = 0;
    let mut more_positions = 0;
    let mut broader_wildcards = 0;
    let mut narrower_wildcards = 0;
    for case in &cases {
        let expected: BTreeMap<_, _> = case.premises.iter().map(|premise| (premise.id.as_u64().expect("id").to_string(), premise.clone())).collect();
        let actual = effective_explicit_needs(&doc, case.command, &case.params);
        let ids: BTreeSet<_> = expected.keys().chain(actual.keys()).collect();
        for id in ids {
            let wanted = expected.get(id);
            let got = actual.get(id);
            for (class, wanted, got) in [
                ("fields", wanted.map(|need| &need.fields), got.map(|need| &need.fields)),
                ("positions", wanted.map(|need| &need.positions), got.map(|need| &need.positions)),
            ] {
                let finite = |values: &[String]| values.iter().cloned().collect::<BTreeSet<_>>();
                let (less, more, broad, narrow) = match (wanted, got) {
                    (Some(None), Some(None)) => (Vec::new(), Vec::new(), false, false),
                    (Some(Some(_)), Some(None)) => (Vec::new(), vec!["*".to_owned()], true, false),
                    (Some(None), _) => (vec!["*".to_owned()], Vec::new(), false, true),
                    (Some(Some(wanted)), Some(Some(got))) => {
                        let (wanted, got) = (finite(wanted), finite(got));
                        (wanted.difference(&got).cloned().collect(), got.difference(&wanted).cloned().collect(), false, false)
                    }
                    (Some(Some(wanted)), None) => (wanted.clone(), Vec::new(), false, false),
                    (None, Some(Some(got))) => (Vec::new(), got.clone(), false, false),
                    (None, Some(None)) => (Vec::new(), vec!["*".to_owned()], true, false),
                    (None, None) => (Vec::new(), Vec::new(), false, false),
                };
                if less.is_empty() && more.is_empty() {
                    continue;
                }
                if broad {
                    broader_wildcards += 1;
                } else if narrow {
                    narrower_wildcards += 1;
                } else if class == "fields" {
                    less_fields += less.len();
                    more_fields += more.len();
                } else {
                    less_positions += less.len();
                    more_positions += more.len();
                }
                differences.push(format!("{} {} {id}.{class}: less={less:?}; more={more:?}; source={}", case.case, case.command, case.source));
            }
        }
    }
    println!(
        "LAYER_FIELD_READINGS cases={} commands={} finite_fields_less={less_fields} finite_fields_more={more_fields} finite_positions_less={less_positions} finite_positions_more={more_positions} broader_wildcard_components={broader_wildcards} narrower_wildcard_components={narrower_wildcards}",
        cases.len(),
        cases.iter().map(|case| case.command).collect::<BTreeSet<_>>().len()
    );
    assert!(differences.is_empty(), "{} field/position component differences:\n{}", differences.len(), differences.join("\n"));
}
