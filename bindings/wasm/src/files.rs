//! The parameters of this kit's commands that name files or folders on disk (KHP A28), declared
//! as typed files in the commands' schemas (`kition_kit::files`): a host lends them, and a
//! headless kit keeps those commands from agents (the host opens, saves and exports documents).

use kition_kit::files::FileParam;

/// Every file or folder parameter of the kit's commands.
/// What `file.import` reads: the media library's movies, stills, audio and 3D models
/// (`kition_motion_media::is_importable`; the test below keeps the two the same).
const IMPORTABLE: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "exr", "psd", "psb", "svg", "pdf", "ai", "eps", "wav",
    "wave", "bwf", "aif", "aiff", "mp3", "mp2", "m4a", "aac", "flac", "ogg", "opus", "gltf", "glb", "obj",
];

pub const FILES: &[FileParam] = &[
    FileParam::read("effect.plugins.load", "path", &["wasm", "wat"]),
    FileParam::folder("effect.plugins.load", "folder"),
    FileParam::create("template.add", "path", &["ectemplate"]),
    FileParam::read("file.openRecent", "path", &[]),
    FileParam::read("file.importRecent", "path", &[]),
    FileParam::read("animation.applyPreset", "path", &[]),
    FileParam::read("animation.applyRecentPreset", "path", &[]),
    FileParam::folder("file.watchFolder", "path"),
    FileParam::create("script.settings.save", "path", &[]),
    FileParam::read("file.open", "path", &[]),
    FileParam::write("file.save", "path", &[]),
    FileParam::create("file.saveAs", "path", &[]),
    FileParam::read("file.import", "paths", IMPORTABLE),
    FileParam::read("roto.model.install", "path", &[]),
    FileParam::read("face.model.install", "path", &[]),
    FileParam::read("layer.newModel", "path", &[]),
    FileParam::create("renderQueue.setOutput", "path", &[]),
    FileParam::create("render.saveCurrentPreview", "path", &[]),
    FileParam::folder("renderQueue.setOverflowFolders", "folders"),
    FileParam::read("view.importGuides", "path", &[]),
    FileParam::create("view.exportGuides", "path", &[]),
    FileParam::create("file.saveCopy", "path", &[]),
    FileParam::read("file.importMultiple", "paths", &[]),
    FileParam::folder("file.collectFiles", "folder"),
    FileParam::read("file.replaceFootage", "path", &[]),
    FileParam::folder("file.relinkFootage", "folder"),
    FileParam::folder("file.watchFolder", "folder"),
    FileParam::create("shortcut.export", "path", &[]),
    FileParam::read("shortcut.import", "path", &[]),
    FileParam::create("file.saveCopyAsXml", "path", &[]),
    FileParam::read("file.executeFile", "path", &[]),
    FileParam::read("keyframe.rpfCameraImport", "path", &[]),
    FileParam::read("templateProps.templateInfo", "path", &[]),
    FileParam::read("file.setProxy", "path", &[]),
    FileParam::folder("mediaBrowser.list", "path"),
    FileParam::folder("mediaBrowser.go", "path"),
    FileParam::folder("mediaBrowser.addFavorite", "path"),
    FileParam::folder("mediaBrowser.removeFavorite", "path"),
    FileParam::read("mediaBrowser.import", "paths", &[]),
    FileParam::read("mediaBrowser.fileInfo", "path", &[]),
    FileParam::read("effect.plugins.load", "path", &["wasm", "wat"]),
    FileParam::create("animation.savePreset", "path", &["ecpreset"]),
    FileParam::read("file.runScript", "path", &["jsx", "js", "jsonl", "json"]),
    FileParam::create("file.exportLottie", "path", &["json", "lottie"]),
    FileParam::read("file.importLottie", "path", &["json", "lottie"]),
    FileParam::read("file.importTimeline", "path", &["xml", "fcpxml", "otio", "edl", "aaf", "omf"]),
    FileParam::create("file.exportTimeline", "path", &["xml", "fcpxml", "otio", "edl", "aaf", "omf"]),
    FileParam::create("comp.saveFrameAs", "path", &["png"]),
    FileParam::create("comp.saveFrameAsPsd", "path", &["psd"]),
    FileParam::create("comp.saveFrameAsExr", "path", &["exr"]),
    FileParam::folder("file.watchFolder.poll", "folder"),
    FileParam::create("templateProps.exportTemplate", "path", &["ectemplate"]),
    FileParam::read("templateProps.importTemplate", "path", &["ectemplate"]),
    FileParam::create("file.addProxy", "path", &[]),
    FileParam::read("file.installScript", "path", &["jsx", "js"]),
    FileParam::read("file.installScriptPanel", "path", &["jsx", "js"]),
];

#[cfg(test)]
mod tests {
    use kition_kit::KitDocument;

    use super::*;

    /// Parameters whose names read like files but are not: ids, property paths, directions,
    /// folders of the kit's own panels, in-memory data.
    const NOT_FILES: &[(&str, &str)] = &[
        // Property paths inside a layer, read through the shared property-path parameter.
        ("edit.pasteReversedKeyframes", "path"),
        ("prop.get", "path"),
        ("prop.set", "path"),
        ("prop.toggleAnimation", "path"),
        ("prop.addKey", "path"),
        ("prop.toggleKey", "path"),
        ("keyframe.toggleTransform", "path"),
        ("prop.setExpression", "path"),
        ("prop.reset", "path"),
        ("prop.select", "path"),
        ("prop.deselect", "path"),
        ("prop.convertExpressionToKeyframes", "path"),
        ("keyframe.paste", "path"),
        ("keyframe.set", "path"),
        ("keyframe.setEase", "path"),
        ("prop.pickWhip", "path"),
        ("path.rotoBezier", "path"),
        ("path.freeTransform", "path"),
        ("keyframe.setSpatialTangents", "path"),
        ("path.pointsFollowNulls", "path"),
        ("path.nullsFollowPoints", "path"),
        ("path.tracePath", "path"),
        ("effect.editDropdown", "path"),
        ("templateProps.canAdd", "path"),
        ("project.move", "folder"),
        ("templateProps.addProperty", "path"),
        // A property path inside a layer (`transform/position`), not a file.
        ("templateProps.linkProperty", "path"),
        ("templateProps.unlinkProperty", "path"),
        ("keyframe.applyEase", "path"),
    ];

    #[test]
    fn every_file_parameter_is_declared() {
        let doc = crate::MotionDocument::create("Untitled.kmotion", 64, 64).unwrap_or_else(|error| panic!("{error}"));
        let problems = kition_kit::files::check(&doc.commands(), FILES, NOT_FILES);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn import_accepts_what_the_media_library_imports() {
        let library: std::collections::BTreeSet<&str> = kition_motion_media::MODEL_EXTENSIONS
            .iter()
            .chain(kition_motion_media::STILL_EXTENSIONS)
            .chain(kition_motion_media::AUDIO_EXTENSIONS)
            .chain(kition_motion_media::VIDEO_EXTENSIONS)
            .copied()
            .collect();
        assert_eq!(super::IMPORTABLE.iter().copied().collect::<std::collections::BTreeSet<_>>(), library);
    }
}
