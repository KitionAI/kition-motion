use super::Row;

macro_rules! text {
    ($command:literal,$path:literal,$reads:expr,$checks:expr,$writes:expr,$state:expr) => {
        Row {
            command: $command,
            path: $path,
            resolves: &[
                "target validates comp explicit/default active; layer explicit id/index/name else text_edit.layer else first selected Text",
                "actual layer_doc/edit_doc read/write use ACTIVE comp and discard resolved comp",
                "ranges are clamped character positions",
            ],
            reads: $reads,
            checks: $checks,
            writes: $writes,
            state: $state,
            source: "commands/text_edit.rs:43,53,100; keyframe/src/text_doc.rs:665,753,765,825",
        }
    };
}
pub(super) const ROWS: &[Row] = &[
    text!(
        "text.edit",
        "start edit",
        &["Text source eligibility", "sourceText at CTI determines character count", "layer timing"],
        &["text layer selected/explicit or already editing"],
        &["text_edit layer/anchor/caret/created/pending", "selected_layers"],
        &["active_comp", "selected_layers", "text_edit", "current time"]
    ),
    text!(
        "text.endEdit",
        "delete created empty text layer",
        &["text_edit.created/layer", "sourceText at CTI empty eligibility", "layer timing"],
        &["no session: no-op", "created and empty only deletion"],
        &["active comp layer removed when eligible", "text_edit removed", "selected_layers remove deleted target"],
        &["active_comp", "text_edit", "selected_layers", "current time"]
    ),
    text!(
        "text.setSelection",
        "editor selection",
        &["sourceText at CTI char count", "previous selection changes pending style reset"],
        &["valid start/end or select all", "Text target"],
        &["text_edit", "selected_layers if new session"],
        &["active_comp", "selected_layers", "text_edit", "current time"]
    ),
    text!(
        "text.moveCaret",
        "character/word/paragraph/document movement",
        &["sampled full sourceText text", "current selection/caret", "paragraph ranges and word boundaries"],
        &["editing session", "valid movement"],
        &["text_edit anchor/caret/goal_x/pending"],
        &["active_comp", "text_edit", "current time"]
    ),
    text!(
        "text.moveCaret",
        "line/vertical layout movement",
        &["full sampled TextDoc formatting/text/layout and font metrics", "current selection/caret/goal_x"],
        &["editing session", "valid movement"],
        &["text_edit anchor/caret/goal_x/pending"],
        &["active_comp", "text_edit", "current time"]
    ),
    text!(
        "text.insert",
        "replace explicit or session range",
        &[
            "full sourceText sampled at CTI; surrounding character styles and paragraph settings preserved",
            "range names observed characters/insertion gap",
            "pending style else replaced first/insertion preceding style",
            "sourceText keys membership/hold_only/destination interpolation",
            "layer timing",
            "old layer.name and first text line for auto-rename eligibility"
        ],
        &["text argument present", "Text target"],
        &["sourceText base/current key", "layer.name if auto-name eligibility", "text_edit caret/anchor/pending/goal_x"],
        &["active_comp", "selected_layers", "text_edit", "current time"]
    ),
    text!(
        "text.delete",
        "delete range/selection or adjacent character/word",
        &[
            "full sourceText sampled at CTI",
            "observed range or caret/word boundary",
            "surrounding character styles and paragraph settings",
            "keys membership/hold_only/destination interpolation",
            "layer timing",
            "layer.name/first line auto-name eligibility"
        ],
        &["Text target", "empty deletion range no-op"],
        &["sourceText base/current key", "conditional layer.name", "text_edit caret/anchor/pending/goal_x"],
        &["active_comp", "selected_layers", "text_edit", "current time"]
    ),
    text!(
        "edit.pasteTextMatchFormatting",
        "not editing: replace target whole text",
        &[
            "clipboard text if text omitted",
            "full sampled sourceText, first-character style and paragraph settings",
            "keys membership/hold_only/interpolation",
            "layer timing/name/first-line eligibility"
        ],
        &["Text target"],
        &["sourceText base/current key", "conditional layer.name"],
        &["active_comp", "selected_layers", "text_edit absent", "text_clipboard", "current time"]
    ),
    text!(
        "edit.pasteTextMatchFormatting",
        "editing: replace selected range",
        &[
            "clipboard text if omitted",
            "full sampled TextDoc",
            "range names characters/gap",
            "pending style else caret/first-range character style",
            "surrounding paragraph settings",
            "keys membership/hold_only/interpolation",
            "layer timing/name/first-line eligibility"
        ],
        &["Text target"],
        &["sourceText base/current key", "conditional layer.name", "text_edit anchor/caret/pending"],
        &["active_comp", "text_edit", "text_clipboard", "current time"]
    ),
    text!(
        "edit.pasteTextFormattingOnly",
        "caret only",
        &["clipboard first character style", "text_edit empty range eligibility"],
        &["clipboard exists", "editing or selected Text"],
        &["text_edit.pending"],
        &["text_edit", "text_clipboard", "selected_layers"]
    ),
    text!(
        "edit.pasteTextFormattingOnly",
        "selected range or selected text layers",
        &[
            "clipboard first character style and first paragraph",
            "full sourceText sampled at CTI preserving text and other formatting",
            "range character and touched paragraph addresses",
            "keys membership/hold_only/interpolation",
            "layer timing/name/first-line auto-name eligibility",
            "Text source filter"
        ],
        &["clipboard exists", "Text targets"],
        &["sourceText base/current key character/paragraph formatting", "conditional layer.name"],
        &["active_comp", "text_edit", "text_clipboard", "selected_layers", "current time"]
    ),
];
