//! Decimal-string encoding of the ticks concurrent edits reason about exactly (a layer's start,
//! in and out points, a keyframe's time); see the crate docs for why a KDM `integer` (or any JSON
//! number) is not safe at every `Tick`.

use kition_motion_time::Tick;
use serde_json::Value;

use crate::{KdmResult, kdm_err};

pub(crate) fn to_value(t: Tick) -> Value {
    Value::String(t.0.to_string())
}

/// Reads a `string`-kind tick register, treating a missing or malformed value as corrupt input
/// (an error the caller reports, never a panic).
pub(crate) fn from_value(value: &Value) -> KdmResult<Tick> {
    let text = value.as_str().ok_or_else(|| kdm_err("expected a decimal-string tick"))?;
    text.parse::<i64>().map(Tick).map_err(kdm_err)
}
