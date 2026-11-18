//! `document.text` (`KitText 1.0`, [`KitDocument::text`](kition_kit::KitDocument::text)): the project's words for a host's
//! full-text search, which the object model does not hold. Read-only: it reads the project and
//! changes nothing.
//!
//! Per composition, in project order, one part per composition labelled with its name: the
//! Source Text of its text layers in layer order (layer #1 first), each a block; blocks are
//! separated by a newline. A Source Text with keyframes gives each keyframe's text in time order
//! (a repeat of the text before it once). A layer whose video is switched off is not drawn and is
//! left out.

use kition_kit::text::{DocumentText, TextBuilder};
use kition_motion_project::{Layer, LayerSource, Project, Value as PropValue};

/// A part's blocks, each one's lines (`\r` or `\n`) trimmed of trailing blanks and blank lines
/// dropped, joined by newlines.
#[derive(Default)]
struct Blocks(String);

impl Blocks {
    fn add(&mut self, text: &str) {
        for line in text.split(['\r', '\n']).map(str::trim_end).filter(|line| !line.trim().is_empty()) {
            if !self.0.is_empty() {
                self.0.push('\n');
            }
            self.0.push_str(line);
        }
    }
}

/// A text layer's words: its Source Text, or each keyframe's in time order.
fn layer_texts(layer: &Layer) -> Vec<&str> {
    if !matches!(layer.source, LayerSource::Text) || !layer.switches.video {
        return Vec::new();
    }
    let Some(source) = layer.props.prop("text/sourceText") else { return Vec::new() };
    fn text(value: &PropValue) -> Option<&str> {
        match value {
            PropValue::Text(doc) => Some(doc.text.as_str()),
            _ => None,
        }
    }
    let mut texts: Vec<&str> =
        if source.keys.is_empty() { text(&source.value).into_iter().collect() } else { source.keys.iter().filter_map(|key| text(&key.value)).collect() };
    texts.dedup();
    texts
}

/// The words of `project`, at most `limit` characters.
pub fn document_text(project: &Project, limit: usize) -> DocumentText {
    let mut builder = TextBuilder::new(limit);
    for (id, comp) in project.comps() {
        let name = project.item(*id).map_or("Composition", |item| item.name.as_str());
        let mut blocks = Blocks::default();
        for layer in &comp.layers {
            for text in layer_texts(layer) {
                blocks.add(text);
            }
        }
        if !builder.part(name, &blocks.0) {
            break;
        }
    }
    builder.finish()
}
