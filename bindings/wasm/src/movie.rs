//! What the editor's Export dialog asks `renderMovie` for: the format, the stretch of the
//! composition (all of it or its work area), the size (full, half, or a width and height) and the
//! frame rate (the composition's, or another). The dialog sends them as a JSON object in place of
//! the format name; a bare format name (`h264`) renders the whole composition as it is.

use serde_json::{Map, Value, json};

/// The formats the Export dialog offers: the ones the web build encodes into one file.
const FORMATS: [&str; 3] = ["h264", "webm", "gif"];
/// The size of a custom output, in pixels on each side.
const SIZE: std::ops::RangeInclusive<u64> = 16..=8192;

/// The `renderQueue.add` parameters that render comp `comp` to `path` with `settings`: a format
/// name, or a JSON object `{format, span: "comp"|"workArea", size: "full"|"half"|{width, height},
/// frameRate: fps|null}`. Anything else is refused with what it takes.
pub(crate) fn queue_params(comp: u64, path: &str, settings: &str) -> Result<Value, String> {
    let settings = settings.trim();
    if !settings.starts_with('{') {
        return Ok(json!({ "comp": comp, "format": if settings.is_empty() { "h264" } else { settings }, "output": path }));
    }
    let given: Map<String, Value> = serde_json::from_str(settings).map_err(|error| format!("the export settings are not a JSON object: {error}"))?;
    let mut params = json!({ "comp": comp, "output": path });
    for (key, value) in &given {
        match (key.as_str(), value) {
            ("format", Value::String(format)) if FORMATS.contains(&format.as_str()) => params["format"] = json!(format),
            ("span", Value::String(span)) if span == "comp" || span == "workArea" => params["timeSpan"] = json!(span),
            ("size", Value::String(size)) if size == "full" || size == "half" => params["resolution"] = json!(size),
            ("size", Value::Object(size)) => {
                let side = |name: &str| size.get(name).and_then(Value::as_u64).filter(|n| SIZE.contains(n));
                let (Some(width), Some(height)) = (side("width"), side("height")) else {
                    return Err(format!("the export size needs a width and a height from {} to {} pixels", SIZE.start(), SIZE.end()));
                };
                params["resize"] = json!({ "enabled": true, "width": width, "height": height, "lockAspect": false });
            }
            ("frameRate", Value::Null) => params["frameRate"] = Value::Null,
            ("frameRate", Value::Number(rate)) if rate.as_f64().is_some_and(|fps| fps.is_finite() && fps > 0.0 && fps <= 240.0) => {
                params["frameRate"] = value.clone()
            }
            _ => {
                return Err(format!(
                    "the export setting `{key}` is {value}: it takes format ({}), span (comp or workArea), size (full, half or {{width, height}}) and frameRate (fps or null)",
                    FORMATS.join(", ")
                ));
            }
        }
    }
    if params.get("format").is_none() {
        params["format"] = json!("h264");
    }
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_name_renders_the_whole_composition() {
        assert_eq!(queue_params(1, "a.mp4", "h264").unwrap(), json!({ "comp": 1, "format": "h264", "output": "a.mp4" }));
        assert_eq!(queue_params(1, "a.mp4", "").unwrap()["format"], "h264");
    }

    #[test]
    fn the_dialogs_settings_become_render_queue_parameters() {
        let params = queue_params(3, "a.webm", r#"{"format":"webm","span":"workArea","size":{"width":320,"height":180},"frameRate":15}"#).unwrap();
        assert_eq!(params["format"], "webm");
        assert_eq!(params["timeSpan"], "workArea");
        assert_eq!(params["resize"], json!({ "enabled": true, "width": 320, "height": 180, "lockAspect": false }));
        assert_eq!(params["frameRate"], 15);
        assert_eq!(queue_params(3, "a.mp4", r#"{"size":"half","frameRate":null}"#).unwrap()["resolution"], "half");
    }

    #[test]
    fn hostile_settings_are_refused_with_what_they_take() {
        for bad in [
            r#"{"format":"exe"}"#,
            r#"{"size":{"width":0,"height":10}}"#,
            r#"{"frameRate":-1}"#,
            r#"{"span":"forever"}"#,
            r#"{"output":"/etc"}"#,
            "{",
            r#"{"size":{"width":1e30,"height":5}}"#,
        ] {
            assert!(queue_params(1, "a.mp4", bad).is_err(), "{bad} should be refused");
        }
    }
}
