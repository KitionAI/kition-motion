//! Footage import for Kition Motion (layer **L3**: it implements `kition_motion_render::FootageSource`).
//!
//! - [`probe`] / [`probe_bytes`]: inspect a file and describe it as an [`kition_motion_project::Footage`]
//!   (video, still, image sequence or audio; size, rate, duration, streams, codec, alpha).
//! - [`MediaPool`]: decodes footage frames on demand into [`kition_motion_raster::Image`]s
//!   (premultiplied f32, sRGB/Rec.709-encoded values, as the compositor expects) with a
//!   memory-budgeted LRU frame cache, one decoder per source, and the decoder kept positioned for
//!   sequential playback. Also audio ([`MediaPool::audio_samples`]) and thumbnails.
//!
//! Video containers (MP4/MOV, Matroska/WebM) and codecs (H.264, HEVC, VP9, AV1, ProRes, DNxHD,
//! MJPEG; AAC, Opus, PCM, MP3/FLAC/Vorbis) come from Kition Film's pure-Rust crates (git dependency,
//! pinned). Kition Film types never leave this crate. Stills use the `image` crate
//! (PNG, JPEG, GIF, WebP, TIFF, BMP, OpenEXR); Photoshop documents (merged image or one layer) come
//! from `kition-motion-psd` and SVG from `kition-motion-svg` (rasterised at any scale).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod convert;
pub mod exr_channels;
mod gif_anim;
mod layered;
pub use layered::vector_doc;
mod pool;
mod probe;

pub use pool::{DEFAULT_BUDGET, MediaPool, PoolStats};
pub use probe::{DEFAULT_SEQUENCE_RATE, probe, probe_bytes, probe_model};

/// Errors from probing or decoding footage.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("unsupported media: {0}")]
    Unsupported(String),
    #[error("decode error: {0}")]
    Decode(String),
}

pub type Result<T> = std::result::Result<T, MediaError>;

impl From<kition_film_media::MediaError> for MediaError {
    fn from(e: kition_film_media::MediaError) -> Self {
        use kition_film_media::MediaError as F;
        match e {
            F::Unsupported(s) => MediaError::Unsupported(s),
            F::Io(s) | F::Offline(s) => MediaError::Io(s),
            other => MediaError::Decode(other.to_string()),
        }
    }
}

/// File extensions recognised as stills.
pub const STILL_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "exr", "psd", "psb", "svg", "pdf", "ai", "eps"];
/// File extensions recognised as audio-only files.
pub const AUDIO_EXTENSIONS: &[&str] = kition_film_media::AUDIO_EXTENSIONS;
/// File extensions recognised as movies.
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "mkv", "webm"];

/// File extensions recognised as 3D models (Advanced 3D model layers).
pub const MODEL_EXTENSIONS: &[&str] = &["gltf", "glb", "obj"];

/// Whether `path` has an extension we can import.
pub fn is_importable(path: &std::path::Path) -> bool {
    let ext = ext_of(path);
    MODEL_EXTENSIONS.contains(&ext.as_str())
        || STILL_EXTENSIONS.contains(&ext.as_str())
        || AUDIO_EXTENSIONS.contains(&ext.as_str())
        || VIDEO_EXTENSIONS.contains(&ext.as_str())
}

pub(crate) fn ext_of(path: &std::path::Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

pub(crate) fn rate_from_fc(r: kition_film_time::FrameRate) -> kition_motion_time::FrameRate {
    if r.num > 0 && r.den > 0 { kition_motion_time::FrameRate::new(r.num, r.den) } else { kition_motion_time::FrameRate::FPS_30 }
}
