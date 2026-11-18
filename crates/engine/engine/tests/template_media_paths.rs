use kition_motion_engine::{Services, Session};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Memory {
    files: Mutex<HashMap<String, Vec<u8>>>,
    writes: Mutex<Vec<String>>,
}

impl Services for Memory {
    fn read_file(&self, path: &str) -> std::io::Result<Vec<u8>> {
        self.files.lock().unwrap().get(path).cloned().ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    fn write_file(&self, path: &str, data: &[u8]) -> std::io::Result<()> {
        self.writes.lock().unwrap().push(path.into());
        self.files.lock().unwrap().insert(path.into(), data.into());
        Ok(())
    }
}

fn archive_with_counter(media: &[&str], next_id: Option<u64>) -> Vec<u8> {
    let mut source = Session::default();
    let comp = source.execute("comp.new", json!({"name":"Template","width":8,"height":8})).unwrap()["comp"].clone();
    if let Some(next_id) = next_id {
        Arc::make_mut(&mut source.project).next_id = next_id;
    }
    let manifest = json!({"format":"kition-motion-template","version":1,"comp":comp,"name":"Template"});
    let mut entries = vec![("manifest.json".into(), serde_json::to_vec(&manifest).unwrap()), ("project.kmotion".into(), source.project.to_json().into_bytes())];
    entries.extend(media.iter().map(|path| ((*path).into(), vec![1, 2, 3])));
    kition_motion_lottie::zip::store(&entries)
}

fn archive(media: &[&str]) -> Vec<u8> {
    archive_with_counter(media, None)
}

#[test]
fn template_media_rejects_parent_and_root_components_before_any_write() {
    for invalid in ["media/../../escape.bin", "media/1/../escape.bin", "media//escape.bin", "media/C:/escape.bin", "media/..\\escape.bin"] {
        let memory = Arc::new(Memory::default());
        memory.files.lock().unwrap().insert("templates/source.ectemplate".into(), archive(&["media/1/safe.bin", invalid]));
        let mut session = Session { services: memory.clone(), host_files_only: true, ..Session::default() };
        let before = session.project.to_json();
        let result = session.execute("templateProps.importTemplate", json!({"path":"templates/source.ectemplate","addToComp":false}));
        assert!(result.is_err(), "accepted {invalid}: {result:?}");
        assert!(memory.writes.lock().unwrap().is_empty(), "extracted media before refusing {invalid}");
        assert_eq!(session.project.to_json(), before, "changed document for {invalid}");
    }
}

#[test]
fn template_media_accepts_nested_normal_components() {
    let memory = Arc::new(Memory::default());
    memory.files.lock().unwrap().insert("templates/source.ectemplate".into(), archive(&["media/1/nested/safe.bin"]));
    let mut session = Session { services: memory.clone(), host_files_only: true, ..Session::default() };
    session.execute("templateProps.importTemplate", json!({"path":"templates/source.ectemplate","addToComp":false})).unwrap();
    assert_eq!(*memory.writes.lock().unwrap(), ["templates/source Media/1/nested/safe.bin"]);
}

#[test]
fn template_id_offset_overflow_refuses_before_media_or_document_writes() {
    let memory = Arc::new(Memory::default());
    memory.files.lock().unwrap().insert("templates/source.ectemplate".into(), archive_with_counter(&["media/1/safe.bin"], Some(u64::MAX)));
    let mut session = Session { services: memory.clone(), host_files_only: true, ..Session::default() };
    let before = session.project.to_json();
    let result = session.execute("templateProps.importTemplate", json!({"path":"templates/source.ectemplate","addToComp":false}));
    assert!(result.is_err(), "accepted overflowing template: {result:?}");
    assert!(memory.writes.lock().unwrap().is_empty(), "extracted media before refusing overflowing ids");
    assert_eq!(session.project.to_json(), before);
}
