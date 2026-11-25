//! The viewer's frames: what the Composition viewer shows of the active composition at the current
//! time (its 3D view, its region of interest, View ▸ Resolution), and the frame cache that keeps
//! the frames already drawn so playback shows them again without rendering.
//!
//! The cache holds frames of one project revision: any edit (a new revision) empties it. A frame
//! is kept under its composition, its time (snapped to the frame), the preview resolution, the 3D
//! view's camera and the region of interest, so changing any of those draws (and keeps) other
//! frames. A frame is kept as rendered, at the preview resolution (a Half frame is a quarter of a
//! Full one; the viewer blows it up when it shows it), and packed without loss (LZ4), so flat
//! motion graphics take a small part of their pixels' size. The bytes counted are the packed
//! bytes really held; the cache keeps at most its budget ([`BUDGET`] unless the editor sets
//! another between [`MIN_BUDGET`] and [`MAX_BUDGET`]), dropping the frames used longest ago.

use std::collections::HashMap;
use std::sync::Arc;

use kition_motion_project::ItemId;
use kition_motion_time::Tick;
use serde_json::{Value, json};

/// The bytes of frames the cache keeps unless the editor sets another budget.
pub const BUDGET: usize = 400 * 1024 * 1024;
/// The least and the most budget the editor may set (it scales with the device's memory).
pub const MIN_BUDGET: usize = 400 * 1024 * 1024;
pub const MAX_BUDGET: usize = 1536 * 1024 * 1024;

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

/// One drawn frame, packed.
#[derive(Clone)]
pub struct Drawn {
    /// The RGBA pixels, LZ4-packed with their length first.
    packed: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    /// When it was last shown ([`FrameCache::clock`]).
    used: u64,
}

impl Drawn {
    fn pack(rgba: &[u8], width: u32, height: u32, used: u64) -> Self {
        Drawn { packed: Arc::new(lz4_flex::block::compress_prepend_size(rgba)), width, height, used }
    }

    /// The frame's RGBA pixels, unpacked; `None` if they do not unpack to its size (which a
    /// frame packed here always does).
    pub fn rgba(&self) -> Option<Vec<u8>> {
        let rgba = lz4_flex::block::decompress_size_prepended(&self.packed).ok()?;
        let expected = (self.width as usize).checked_mul(self.height as usize)?.checked_mul(4)?;
        (rgba.len() == expected).then_some(rgba)
    }

    /// The bytes it holds.
    fn bytes(&self) -> usize {
        self.packed.len()
    }
}

/// The frames of one revision, and what playback measured.
pub struct FrameCache {
    /// The most bytes it keeps ([`BUDGET`] unless set).
    budget: usize,
    revision: u64,
    frames: HashMap<FrameKey, Drawn>,
    /// Packed bytes held, and what the same frames would take unpacked.
    bytes: usize,
    raw: usize,
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
        FrameCache { budget, revision: 0, frames: HashMap::new(), bytes: 0, raw: 0, clock: 0, last: None, shown: 0, rendered: 0, hits: 0, render_ms: 0.0 }
    }

    /// Sets the most bytes it keeps, between [`MIN_BUDGET`] and [`MAX_BUDGET`], dropping the
    /// frames used longest ago if it holds more; answers the budget now.
    pub fn set_budget(&mut self, budget: usize) -> usize {
        self.budget = budget.clamp(MIN_BUDGET, MAX_BUDGET);
        self.shrink();
        self.budget
    }

    /// The most bytes it keeps.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Forgets the frames of an older revision.
    fn keep_revision(&mut self, revision: u64) {
        if self.revision != revision {
            self.frames.clear();
            self.bytes = 0;
            self.raw = 0;
            self.revision = revision;
        }
    }

    fn insert(&mut self, key: FrameKey, drawn: Drawn) {
        let raw = (drawn.width as usize).saturating_mul(drawn.height as usize).saturating_mul(4);
        let size = drawn.bytes();
        if let Some(old) = self.frames.insert(key, drawn) {
            self.bytes = self.bytes.saturating_sub(old.bytes());
            self.raw = self.raw.saturating_sub((old.width as usize).saturating_mul(old.height as usize).saturating_mul(4));
        }
        self.bytes = self.bytes.saturating_add(size);
        self.raw = self.raw.saturating_add(raw);
    }

    /// Drops the frames used longest ago while it is over its budget.
    fn shrink(&mut self) {
        while self.bytes > self.budget {
            let Some(oldest) = self.frames.iter().min_by_key(|(_, drawn)| drawn.used).map(|(key, _)| key.clone()) else { break };
            if let Some(gone) = self.frames.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(gone.bytes());
                self.raw = self.raw.saturating_sub((gone.width as usize).saturating_mul(gone.height as usize).saturating_mul(4));
            }
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
    pub fn put(&mut self, revision: u64, key: FrameKey, rgba: &[u8], width: u32, height: u32, ms: f64) {
        self.keep_revision(revision);
        self.clock = self.clock.wrapping_add(1);
        self.shown = self.shown.saturating_add(1);
        self.rendered = self.rendered.saturating_add(1);
        if ms.is_finite() && ms >= 0.0 {
            self.render_ms += ms;
        }
        self.last = Some(key.clone());
        let drawn = Drawn::pack(rgba, width, height, self.clock);
        if drawn.bytes() > self.budget {
            return;
        }
        self.insert(key, drawn);
        self.shrink();
    }

    /// Whether it holds the frame for `key` at `revision` (not counted as shown).
    pub fn holds(&self, revision: u64, key: &FrameKey) -> bool {
        self.revision == revision && self.frames.contains_key(key)
    }

    /// Keeps a frame rendered ahead (Cache Work Area in Background): not counted as shown, and
    /// only while it fits the budget without dropping a frame of the same settings (frames kept
    /// for other settings, another preview resolution say, go first, those used longest ago
    /// first); answers whether it was kept.
    pub fn put_ahead(&mut self, revision: u64, key: FrameKey, rgba: &[u8], width: u32, height: u32, ms: f64) -> bool {
        self.keep_revision(revision);
        let drawn = Drawn::pack(rgba, width, height, self.clock.wrapping_add(1));
        while self.bytes.saturating_add(drawn.bytes()) > self.budget {
            let other =
                self.frames.iter().filter(|(each, _)| each.settings() != key.settings()).min_by_key(|(_, kept)| kept.used).map(|(each, _)| each.clone());
            let Some(other) = other else { return false };
            if let Some(gone) = self.frames.remove(&other) {
                self.bytes = self.bytes.saturating_sub(gone.bytes());
                self.raw = self.raw.saturating_sub((gone.width as usize).saturating_mul(gone.height as usize).saturating_mul(4));
            }
        }
        self.clock = self.clock.wrapping_add(1);
        if ms.is_finite() && ms >= 0.0 {
            self.render_ms += ms;
        }
        self.insert(key, drawn);
        true
    }

    /// Forgets which frame was shown last, so the next one found counts as shown (tests).
    #[cfg(test)]
    pub fn clear_last_for_tests(&mut self) {
        self.last = None;
    }

    /// Empties the cache (View ▸ Purge).
    pub fn clear(&mut self) {
        self.frames.clear();
        self.bytes = 0;
        self.raw = 0;
        self.last = None;
    }

    /// What the editor shows of the cache: the frames kept for the settings of `now` at
    /// `revision` (frame numbers at `fps`, from the composition's start), the bytes kept (packed,
    /// as held, and what they would take unpacked), the budget, and the counters playback reads
    /// its frame rate from.
    pub fn report(&self, revision: u64, now: Option<&FrameKey>, frame_of: impl Fn(Tick) -> i64) -> Value {
        let mut cached: Vec<i64> = match now {
            Some(now) if self.revision == revision => self.frames.keys().filter(|key| key.settings() == now.settings()).map(|key| frame_of(key.time)).collect(),
            _ => Vec::new(),
        };
        cached.sort_unstable();
        cached.dedup();
        let current = self.revision == revision;
        json!({
            "cached": cached,
            "bytes": if current { self.bytes } else { 0 },
            "rawBytes": if current { self.raw } else { 0 },
            "budget": self.budget,
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

    /// `n` bytes that do not pack (a simple generator's noise), so a frame takes about its size.
    fn noise(n: usize, seed: u32) -> Vec<u8> {
        let mut x = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x >> 24) as u8
            })
            .collect()
    }

    /// The bytes a frame of `rgba` takes in the cache.
    fn held(rgba: &[u8]) -> usize {
        Drawn::pack(rgba, 1, 1, 0).bytes()
    }

    #[test]
    fn a_new_revision_empties_the_cache() {
        let mut cache = FrameCache::default();
        cache.put(1, key(0), &[0; 16], 2, 2, 1.0);
        assert!(cache.get(1, &key(0)).is_some());
        assert!(cache.get(2, &key(0)).is_none());
        assert_eq!(cache.report(2, Some(&key(0)), |t| t.0)["cached"], json!([]));
    }

    #[test]
    fn the_report_lists_the_frames_of_the_current_settings() {
        let mut cache = FrameCache::default();
        for t in [3, 1, 2] {
            cache.put(7, key(t), &[0; 16], 2, 2, 1.0);
        }
        let other = FrameKey { preview: 2, ..key(9) };
        cache.put(7, other, &[0; 16], 2, 2, 1.0);
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
        cache.put(1, key(0), &noise(big, 0), 1, 1, 1.0);
        cache.put(1, key(1), &noise(big, 1), 1, 1, 1.0);
        assert!(cache.get(1, &key(0)).is_some());
        cache.put(1, key(2), &noise(big, 2), 1, 1, 1.0);
        assert!(cache.bytes <= SMALL);
        assert!(cache.get(1, &key(1)).is_none(), "the frame used longest ago went");
        assert!(cache.get(1, &key(0)).is_some() && cache.get(1, &key(2)).is_some());
        cache.put(1, key(3), &noise(SMALL + 1, 3), 1, 1, 1.0);
        assert!(cache.get(1, &key(3)).is_none(), "a frame larger than the whole budget is not kept");
    }

    #[test]
    fn frames_rendered_ahead_stop_at_the_budget_and_drop_nothing() {
        let frames: Vec<Vec<u8>> = (0..4).map(|seed| noise(1000, seed)).collect();
        // Exactly three of them fit.
        let small: usize = frames.iter().take(3).map(|rgba| held(rgba)).sum();
        let mut cache = FrameCache::with_budget(small);
        for (t, rgba) in frames.iter().enumerate().take(3) {
            assert!(cache.put_ahead(1, key(t as i64), rgba, 1, 1, 1.0));
        }
        assert!(!cache.put_ahead(1, key(3), &frames[3], 1, 1, 1.0), "over the budget: not kept");
        assert!((0..3).all(|t| cache.holds(1, &key(t))) && !cache.holds(1, &key(3)));
        assert_eq!(cache.shown, 0, "frames rendered ahead were not shown");
        assert!(!cache.holds(2, &key(0)), "another revision holds none of them");
    }

    #[test]
    fn frames_rendered_ahead_make_room_by_dropping_frames_of_other_settings_only() {
        let frames: Vec<Vec<u8>> = (0..4).map(|seed| noise(1000, seed)).collect();
        let small: usize = frames.iter().take(2).map(|rgba| held(rgba)).sum();
        let mut cache = FrameCache::with_budget(small + 10);
        // Two frames at Full fill it.
        assert!(cache.put_ahead(1, key(0), &frames[0], 1, 1, 1.0) && cache.put_ahead(1, key(1), &frames[1], 1, 1, 1.0));
        // At Half they make room: the Full frames go, the Half ones stay.
        let half = |t: i64| FrameKey { preview: 2, ..key(t) };
        assert!(cache.put_ahead(1, half(0), &frames[2], 1, 1, 1.0), "a Full frame made room");
        assert!(cache.put_ahead(1, half(1), &frames[3], 1, 1, 1.0), "the other Full frame made room");
        assert!(!cache.holds(1, &key(0)) && !cache.holds(1, &key(1)));
        assert!(!cache.put_ahead(1, half(2), &frames[0], 1, 1, 1.0), "full of Half frames: none of them is dropped");
        assert!(cache.holds(1, &half(0)) && cache.holds(1, &half(1)) && cache.bytes <= small + 10);
    }

    #[test]
    fn a_frame_comes_back_pixel_for_pixel_and_the_bytes_counted_are_the_bytes_held() {
        let (w, h) = (64u32, 36u32);
        // Flat colour with a noisy band: some of it packs, some does not.
        let mut rgba = [20u8, 40, 200, 255].repeat((w * h) as usize);
        let band = noise((w * 4 * 6) as usize, 9);
        rgba[(w * 4 * 10) as usize..(w * 4 * 16) as usize].copy_from_slice(&band);
        let mut cache = FrameCache::default();
        cache.put(1, key(0), &rgba, w, h, 1.0);
        let back = cache.get(1, &key(0)).and_then(|drawn| drawn.rgba());
        assert_eq!(back.as_deref(), Some(&rgba[..]), "the same pixels come back");
        let report = cache.report(1, Some(&key(0)), |t| t.0);
        assert_eq!(report["bytes"].as_u64(), Some(held(&rgba) as u64), "the packed bytes are counted");
        assert_eq!(report["rawBytes"].as_u64(), Some(u64::from(w * h * 4)));
        assert!(report["bytes"].as_u64() < report["rawBytes"].as_u64(), "flat colour packs");
    }

    #[test]
    fn a_flat_full_hd_frame_takes_a_small_part_of_its_pixels() {
        let rgba = [0u8, 0, 0, 255].repeat(1920 * 1080);
        let size = held(&rgba);
        assert!(size * 100 < rgba.len(), "a flat 1920 × 1080 frame packs to {size} bytes");
        // So the 300 frames of a default composition fit the smallest budget many times over.
        assert!(size * 300 < MIN_BUDGET);
    }

    #[test]
    fn the_budget_set_is_kept_between_its_limits_and_a_smaller_one_drops_frames() {
        let mut cache = FrameCache::default();
        assert_eq!(cache.set_budget(1), MIN_BUDGET);
        assert_eq!(cache.set_budget(usize::MAX), MAX_BUDGET);
        assert_eq!(cache.set_budget(800 * 1024 * 1024), 800 * 1024 * 1024);
        assert_eq!(cache.report(0, None, |t| t.0)["budget"].as_u64(), Some(800 * 1024 * 1024));
        let mut small = FrameCache::with_budget(10_000);
        small.put(1, key(0), &noise(4000, 1), 1, 1, 1.0);
        small.put(1, key(1), &noise(4000, 2), 1, 1, 1.0);
        small.budget = 5000;
        small.shrink();
        assert!(small.bytes <= 5000 && small.holds(1, &key(1)) && !small.holds(1, &key(0)));
    }
}
