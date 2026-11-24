//! The viewer's frames: what the Composition viewer shows of the active composition at the current
//! time (its 3D view, its region of interest, View ▸ Resolution), and the frame cache that keeps
//! the frames already drawn so playback shows them again without rendering.
//!
//! The cache holds frames of one project revision: any edit (a new revision) empties it. A frame
//! is kept under its composition, its time (snapped to the frame), the preview resolution, the 3D
//! view's camera and the region of interest, so changing any of those draws (and keeps) other
//! frames. It keeps at most [`BUDGET`] bytes, dropping the frames used longest ago.

use std::collections::HashMap;
use std::sync::Arc;

use kition_motion_project::ItemId;
use kition_motion_time::Tick;
use serde_json::{Value, json};

/// The most bytes of frames the cache keeps (about 48 frames of 1920 × 1080).
pub const BUDGET: usize = 400 * 1024 * 1024;

/// What a frame depends on besides the project: the composition, its frame's time, the preview
/// divisor, the 3D view's camera (bit patterns of its eye, view matrix and zoom; empty for the
/// active camera) and the region of interest (bit patterns; empty for none).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameKey {
    pub comp: ItemId,
    pub time: Tick,
    pub preview: u32,
    pub view: Vec<u64>,
    pub roi: Vec<u64>,
}

impl FrameKey {
    /// The same settings at another time.
    fn settings(&self) -> (ItemId, u32, &[u64], &[u64]) {
        (self.comp, self.preview, &self.view, &self.roi)
    }
}

/// One drawn frame.
#[derive(Clone)]
pub struct Drawn {
    pub rgba: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    /// When it was last shown ([`FrameCache::clock`]).
    used: u64,
}

/// The frames of one revision, and what playback measured.
pub struct FrameCache {
    /// The most bytes it keeps ([`BUDGET`]).
    budget: usize,
    revision: u64,
    frames: HashMap<FrameKey, Drawn>,
    bytes: usize,
    clock: u64,
    /// The last frame shown: a frame shown again (a repaint, a scroll) is not counted twice.
    last: Option<FrameKey>,
    /// Frames shown (each change of frame counts once), of them rendered, and found in the cache.
    pub shown: u64,
    pub rendered: u64,
    pub hits: u64,
    /// Milliseconds the rendered frames took, all together.
    pub render_ms: f64,
}

impl Default for FrameCache {
    fn default() -> Self {
        FrameCache::with_budget(BUDGET)
    }
}

impl FrameCache {
    /// An empty cache keeping at most `budget` bytes.
    pub fn with_budget(budget: usize) -> Self {
        FrameCache { budget, revision: 0, frames: HashMap::new(), bytes: 0, clock: 0, last: None, shown: 0, rendered: 0, hits: 0, render_ms: 0.0 }
    }

    /// Forgets the frames of an older revision.
    fn keep_revision(&mut self, revision: u64) {
        if self.revision != revision {
            self.frames.clear();
            self.bytes = 0;
            self.revision = revision;
        }
    }

    /// The frame kept for `key` at `revision`, counted as shown.
    pub fn get(&mut self, revision: u64, key: &FrameKey) -> Option<Drawn> {
        self.keep_revision(revision);
        self.clock = self.clock.wrapping_add(1);
        let clock = self.clock;
        let found = self.frames.get_mut(key).map(|drawn| {
            drawn.used = clock;
            drawn.clone()
        })?;
        if self.last.as_ref() != Some(key) {
            self.shown = self.shown.saturating_add(1);
            self.hits = self.hits.saturating_add(1);
            self.last = Some(key.clone());
        }
        Some(found)
    }

    /// Keeps a frame just rendered (in `ms` milliseconds), dropping the frames used longest ago
    /// while the cache is over its budget.
    pub fn put(&mut self, revision: u64, key: FrameKey, rgba: Arc<Vec<u8>>, width: u32, height: u32, ms: f64) {
        self.keep_revision(revision);
        self.clock = self.clock.wrapping_add(1);
        self.shown = self.shown.saturating_add(1);
        self.rendered = self.rendered.saturating_add(1);
        if ms.is_finite() && ms >= 0.0 {
            self.render_ms += ms;
        }
        self.last = Some(key.clone());
        let size = rgba.len();
        if size > self.budget {
            return;
        }
        if let Some(old) = self.frames.insert(key, Drawn { rgba, width, height, used: self.clock }) {
            self.bytes = self.bytes.saturating_sub(old.rgba.len());
        }
        self.bytes = self.bytes.saturating_add(size);
        while self.bytes > self.budget {
            let Some(oldest) = self.frames.iter().min_by_key(|(_, drawn)| drawn.used).map(|(key, _)| key.clone()) else { break };
            if let Some(gone) = self.frames.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(gone.rgba.len());
            }
        }
    }

    /// Whether it holds the frame for `key` at `revision` (not counted as shown).
    pub fn holds(&self, revision: u64, key: &FrameKey) -> bool {
        self.revision == revision && self.frames.contains_key(key)
    }

    /// Keeps a frame rendered ahead (Cache Work Area in Background): not counted as shown, and
    /// only while it fits the budget without dropping a frame; answers whether it was kept.
    pub fn put_ahead(&mut self, revision: u64, key: FrameKey, rgba: Arc<Vec<u8>>, width: u32, height: u32, ms: f64) -> bool {
        self.keep_revision(revision);
        let size = rgba.len();
        if self.bytes.saturating_add(size) > self.budget {
            return false;
        }
        self.clock = self.clock.wrapping_add(1);
        if ms.is_finite() && ms >= 0.0 {
            self.render_ms += ms;
        }
        if let Some(old) = self.frames.insert(key, Drawn { rgba, width, height, used: self.clock }) {
            self.bytes = self.bytes.saturating_sub(old.rgba.len());
        }
        self.bytes = self.bytes.saturating_add(size);
        true
    }

    /// Empties the cache (View ▸ Purge).
    pub fn clear(&mut self) {
        self.frames.clear();
        self.bytes = 0;
        self.last = None;
    }

    /// What the editor shows of the cache: the frames kept for the settings of `now` at
    /// `revision` (frame numbers at `fps`, from the composition's start), the bytes kept, and the
    /// counters playback reads its frame rate from.
    pub fn report(&self, revision: u64, now: Option<&FrameKey>, frame_of: impl Fn(Tick) -> i64) -> Value {
        let mut cached: Vec<i64> = match now {
            Some(now) if self.revision == revision => self.frames.keys().filter(|key| key.settings() == now.settings()).map(|key| frame_of(key.time)).collect(),
            _ => Vec::new(),
        };
        cached.sort_unstable();
        cached.dedup();
        json!({
            "cached": cached,
            "bytes": if self.revision == revision { self.bytes } else { 0 },
            "shown": self.shown,
            "rendered": self.rendered,
            "hits": self.hits,
            "renderMs": self.render_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(time: i64) -> FrameKey {
        FrameKey { comp: ItemId(1), time: Tick(time), preview: 1, view: Vec::new(), roi: Vec::new() }
    }

    #[test]
    fn a_new_revision_empties_the_cache() {
        let mut cache = FrameCache::default();
        cache.put(1, key(0), Arc::new(vec![0; 16]), 2, 2, 1.0);
        assert!(cache.get(1, &key(0)).is_some());
        assert!(cache.get(2, &key(0)).is_none());
        assert_eq!(cache.report(2, Some(&key(0)), |t| t.0)["cached"], json!([]));
    }

    #[test]
    fn the_report_lists_the_frames_of_the_current_settings() {
        let mut cache = FrameCache::default();
        for t in [3, 1, 2] {
            cache.put(7, key(t), Arc::new(vec![0; 16]), 2, 2, 1.0);
        }
        let other = FrameKey { preview: 2, ..key(9) };
        cache.put(7, other, Arc::new(vec![0; 16]), 2, 2, 1.0);
        assert_eq!(cache.report(7, Some(&key(0)), |t| t.0)["cached"], json!([1, 2, 3]));
        assert!(cache.get(7, &key(2)).is_some());
        let report = cache.report(7, Some(&key(0)), |t| t.0);
        assert_eq!((report["rendered"].as_u64(), report["hits"].as_u64(), report["shown"].as_u64()), (Some(4), Some(1), Some(5)));
    }

    #[test]
    fn it_keeps_within_its_budget_dropping_the_frames_used_longest_ago() {
        const SMALL: usize = 3000;
        let mut cache = FrameCache::with_budget(SMALL);
        let big = SMALL / 3 + 1;
        cache.put(1, key(0), Arc::new(vec![0; big]), 1, 1, 1.0);
        cache.put(1, key(1), Arc::new(vec![0; big]), 1, 1, 1.0);
        assert!(cache.get(1, &key(0)).is_some());
        cache.put(1, key(2), Arc::new(vec![0; big]), 1, 1, 1.0);
        assert!(cache.bytes <= SMALL);
        assert!(cache.get(1, &key(1)).is_none(), "the frame used longest ago went");
        assert!(cache.get(1, &key(0)).is_some() && cache.get(1, &key(2)).is_some());
        cache.put(1, key(3), Arc::new(vec![0; SMALL + 1]), 1, 1, 1.0);
        assert!(cache.get(1, &key(3)).is_none(), "a frame larger than the whole budget is not kept");
    }

    #[test]
    fn frames_rendered_ahead_stop_at_the_budget_and_drop_nothing() {
        const SMALL: usize = 3000;
        let mut cache = FrameCache::with_budget(SMALL);
        let size = SMALL / 3;
        assert!(cache.put_ahead(1, key(0), Arc::new(vec![0; size]), 1, 1, 1.0));
        assert!(cache.put_ahead(1, key(1), Arc::new(vec![0; size]), 1, 1, 1.0));
        assert!(cache.put_ahead(1, key(2), Arc::new(vec![0; size]), 1, 1, 1.0));
        assert!(!cache.put_ahead(1, key(3), Arc::new(vec![0; size]), 1, 1, 1.0), "over the budget: not kept");
        assert!((0..3).all(|t| cache.holds(1, &key(t))) && !cache.holds(1, &key(3)));
        assert_eq!(cache.shown, 0, "frames rendered ahead were not shown");
        assert!(!cache.holds(2, &key(0)), "another revision holds none of them");
    }
}
