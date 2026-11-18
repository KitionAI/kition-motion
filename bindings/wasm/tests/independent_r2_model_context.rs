use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;

#[test]
fn model_choice_installation_and_download_read_no_composition_context() {
    let document = MotionDocument::create("Model.kmotion", 8, 8).unwrap();
    let commands = document.commands();
    for id in ["face.model.select", "face.model.install", "face.model.download", "roto.model.select", "roto.model.install", "roto.model.download"] {
        let command = commands.iter().find(|command| command.id == id).unwrap();
        assert!(command.reads.is_empty(), "{id} reads model registry/preferences/status/files only: {:?}", command.reads);
    }
}
