//! Wiggler, Smoother and Motion Sketch (`keys.wiggle`, `keys.smooth`, `motion.sketch`).

use kition_motion_keyframe::evaluate;
use serde_json::json;

use crate::tests_timeline::{animate, prop, setup};

fn select(s: &mut crate::Session, layer: u64, path: &str) {
    s.execute("prop.select", json!({"layer": layer, "path": path})).unwrap();
}

#[test]
fn wiggle_adds_keys_at_frequency_within_magnitude() {
    let (mut s, l) = setup();
    animate(&mut s, l, "transform/position", &[(0.0, json!([100, 100])), (2.0, json!([300, 100]))]);
    let orig = prop(&s, l, "transform/position");
    select(&mut s, l, "transform/position");
    let opts = json!({"apply": "spatial", "noise": "jagged", "dimensions": "independent", "frequency": 5, "magnitude": 20, "seed": 7});
    let r = s.execute("keys.wiggle", opts.clone()).unwrap();
    let p = prop(&s, l, "transform/position");
    // 2 s at 5 keys/s: 9 keys between the two originals (one every 6 frames at 30 fps).
    assert_eq!(p.keys.len(), 11, "{r}");
    assert_eq!(r["after"], 11);
    let step = p.keys[2].time - p.keys[1].time;
    assert!((step.seconds() - 0.2).abs() < 1e-9);
    for k in &p.keys[1..10] {
        let base = evaluate(&orig.keys, k.time, true).unwrap().as_vec2();
        let v = k.value.as_vec2();
        assert!((v[0] - base[0]).abs() <= 20.0 + 1e-9 && (v[1] - base[1]).abs() <= 20.0 + 1e-9, "{v:?} vs {base:?}");
    }
    assert!(p.keys[1..10].iter().any(|k| (k.value.as_vec2()[1] - 100.0).abs() > 1.0), "noise was applied");
    // End keys untouched; deterministic for the same seed; undo restores.
    assert_eq!(p.keys[0].value, orig.keys[0].value);
    assert_eq!(p.keys[10].value, orig.keys[1].value);
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(prop(&s, l, "transform/position").keys.len(), 2);
    select(&mut s, l, "transform/position");
    s.execute("keys.wiggle", opts).unwrap();
    assert_eq!(prop(&s, l, "transform/position").keys, p.keys);
}

#[test]
fn wiggle_one_dimension_and_smooth_noise() {
    let (mut s, l) = setup();
    animate(&mut s, l, "transform/position", &[(0.0, json!([100, 100])), (1.0, json!([100, 100]))]);
    select(&mut s, l, "transform/position");
    s.execute("keys.wiggle", json!({"dimensions": "one", "dimension": 1, "frequency": 10, "magnitude": 50})).unwrap();
    let p = prop(&s, l, "transform/position");
    assert_eq!(p.keys.len(), 11);
    for k in &p.keys {
        let v = k.value.as_vec2();
        assert!((v[0] - 100.0).abs() < 1e-9);
        assert!((v[1] - 100.0).abs() <= 50.0);
    }
    // Opacity (non-spatial): the temporal path deviates the value.
    animate(&mut s, l, "transform/opacity", &[(0.0, json!(50)), (1.0, json!(50))]);
    select(&mut s, l, "transform/opacity");
    s.execute("keys.wiggle", json!({"apply": "temporal", "frequency": 4, "magnitude": 10})).unwrap();
    let o = prop(&s, l, "transform/opacity");
    assert!(o.keys.len() >= 5);
    assert!(o.keys.iter().all(|k| (k.value.as_f64() - 50.0).abs() <= 10.0));
    assert!(s.execute("keys.wiggle", json!({"noise": "wobbly"})).is_err());
}

#[test]
fn smoother_reduces_keys_within_tolerance() {
    let (mut s, l) = setup();
    // A ramp keyed every frame with a little jitter.
    let keys: Vec<(f64, serde_json::Value)> = (0..=60).map(|f| (f as f64 / 30.0, json!(f as f64 * 1.5 + if f % 2 == 0 { 0.2 } else { -0.2 }))).collect();
    animate(&mut s, l, "transform/opacity", &keys);
    let orig = prop(&s, l, "transform/opacity");
    assert_eq!(orig.keys.len(), 61);
    select(&mut s, l, "transform/opacity");
    let r = s.execute("keys.smooth", json!({"tolerance": 1.0})).unwrap();
    let p = prop(&s, l, "transform/opacity");
    assert!(p.keys.len() < 10, "{} keys ({r})", p.keys.len());
    for f in 0..=60 {
        let t = s.active_comp().unwrap().frame_rate.tick_of(f);
        let a = evaluate(&orig.keys, t, false).unwrap().as_f64();
        let b = evaluate(&p.keys, t, false).unwrap().as_f64();
        assert!((a - b).abs() <= 1.0 + 1e-6, "frame {f}: {a} vs {b}");
    }
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(prop(&s, l, "transform/opacity").keys.len(), 61);
}

/// Smoother with part of a property's keyframes selected: the keyframes outside the selected
/// span (before its first and after its last selected key) are kept as they were, whatever the
/// selection's order and whether it has gaps, and a second run with the same tolerance leaves them
/// alone too. (Inside the span it keeps the fewest keys that hold the curve within the tolerance,
/// never more than were there: they replace every key there, selected or not.)
#[test]
fn smoother_never_touches_keys_outside_the_selected_span() {
    let keys = [(0.0, json!([100, 100])), (0.6, json!([400, 120])), (0.8, json!([150, 300])), (1.5, json!([500, 500])), (2.0, json!([100, 400]))];
    for (order, tolerance) in [([0usize, 2, 3], 5.0), ([3, 0, 2], 5.0), ([1, 2, 3], 0.1), ([0, 1, 3], 1.0)] {
        let (mut s, l) = setup();
        animate(&mut s, l, "transform/position", &keys);
        let orig = prop(&s, l, "transform/position");
        let (first, last) = (order.iter().map(|&i| orig.keys[i].time).min().unwrap(), order.iter().map(|&i| orig.keys[i].time).max().unwrap());
        let outside = |p: &kition_motion_project::Property| {
            p.keys.iter().filter(|k| k.time < first || k.time > last).map(|k| (k.time, k.value.clone())).collect::<Vec<_>>()
        };
        let ends = |p: &kition_motion_project::Property| {
            p.keys.iter().filter(|k| k.time == first || k.time == last).map(|k| (k.time, k.value.clone())).collect::<Vec<_>>()
        };
        let select = |s: &mut crate::Session, times: Vec<f64>| {
            let picked: Vec<_> = times.into_iter().map(|t| json!({"layer": l, "path": "transform/position", "time": t})).collect();
            s.execute("keys.select", json!({"keys": picked})).unwrap();
        };
        select(&mut s, order.iter().map(|&i| orig.keys[i].time.seconds()).collect());
        s.execute("keys.smooth", json!({"tolerance": tolerance})).unwrap();
        let once = prop(&s, l, "transform/position");
        assert_eq!(outside(&once), outside(&orig), "{order:?}: the keyframes outside the span are kept");
        assert_eq!(ends(&once), ends(&orig), "{order:?}: the span's ends are kept");
        let inside = |p: &kition_motion_project::Property| p.keys.iter().filter(|k| k.time >= first && k.time <= last).count();
        assert!(inside(&once) <= inside(&orig), "{order:?}: no more keys than were there");
        // Again, with the same tolerance, over the same span.
        select(&mut s, once.keys.iter().filter(|k| k.time >= first && k.time <= last).map(|k| k.time.seconds()).collect());
        s.execute("keys.smooth", json!({"tolerance": tolerance})).unwrap();
        let twice = prop(&s, l, "transform/position");
        assert_eq!(outside(&twice), outside(&orig), "{order:?}: a second run keeps them too");
        assert_eq!(ends(&twice), ends(&orig));
    }
}

/// The X of `p`'s motion every frame from `from` to `to` seconds (30 fps).
fn xs(s: &crate::Session, p: &kition_motion_project::Property, from: f64, to: f64) -> Vec<(f64, f64)> {
    let fr = s.active_comp().unwrap().frame_rate;
    ((from * 30.0).round() as i64..=(to * 30.0).round() as i64)
        .map(|f| {
            let v = evaluate(&p.keys, fr.tick_of(f), true).unwrap().as_vec2();
            (v[0], v[1])
        })
        .collect()
}

/// Four Position keys on one horizontal line, unevenly spaced (the hand test's 200, 300, 340, 900).
const LINE: [(f64, [f64; 2]); 4] = [(0.0, [200.0, 360.0]), (1.0, [300.0, 360.0]), (1.4, [340.0, 360.0]), (2.5, [900.0, 360.0])];

/// The Smoother never gives more keys than it was given in the span, whatever the tolerance and
/// whether the path through the keys was straight or the default auto-Bezier one (which on these
/// unevenly spaced keys runs back between 300 and 340: before, the Smoother followed that with up
/// to 11 keys).
#[test]
fn smoother_never_adds_keys() {
    for straight in [true, false] {
        for tolerance in [0.0, 0.1, 1.0, 5.0, 50.0] {
            let (mut s, l) = setup();
            s.prefs.general.default_spatial_linear = straight;
            animate(&mut s, l, "transform/position", &LINE.map(|(t, v)| (t, json!(v))));
            select(&mut s, l, "transform/position");
            let r = s.execute("keys.smooth", json!({"tolerance": tolerance})).unwrap();
            let after = prop(&s, l, "transform/position").keys.len();
            assert!(after <= 4, "straight {straight}, tolerance {tolerance}: {after} keys ({r})");
            assert!(r["after"].as_u64() <= r["before"].as_u64(), "{r}");
        }
    }
    // Opacity keyed on a zigzag: no more keys either.
    let (mut s, l) = setup();
    animate(&mut s, l, "transform/opacity", &[(0.0, json!(0)), (0.3, json!(90)), (0.5, json!(10)), (0.9, json!(80)), (1.0, json!(20))]);
    select(&mut s, l, "transform/opacity");
    s.execute("keys.smooth", json!({"tolerance": 0.5})).unwrap();
    assert!(prop(&s, l, "transform/opacity").keys.len() <= 5);
}

/// Keys on a straight line with linear interpolation: the smoothed motion stays on the line and
/// moves one way only, inside the keys' range (no running back past a key, no overshoot), at every
/// tolerance. The default auto-Bezier path of the same keys, smoothed, no longer runs back either.
#[test]
fn smoothing_collinear_keys_stays_on_the_line_and_moves_one_way() {
    for straight in [true, false] {
        for tolerance in [0.1, 1.0, 5.0, 40.0] {
            let (mut s, l) = setup();
            s.prefs.general.default_spatial_linear = straight;
            animate(&mut s, l, "transform/position", &LINE.map(|(t, v)| (t, json!(v))));
            select(&mut s, l, "transform/position");
            s.execute("keys.smooth", json!({"tolerance": tolerance})).unwrap();
            let p = prop(&s, l, "transform/position");
            let path = xs(&s, &p, 0.0, 2.5);
            for (i, (x, y)) in path.iter().enumerate() {
                assert!((y - 360.0).abs() < 1e-6, "straight {straight}, tolerance {tolerance}: frame {i} left the line ({y})");
                assert!((200.0 - 1e-6..=900.0 + 1e-6).contains(x), "straight {straight}, tolerance {tolerance}: frame {i} overshoots ({x})");
            }
            for (i, pair) in path.windows(2).enumerate() {
                assert!(pair[1].0 >= pair[0].0 - 1e-6, "straight {straight}, tolerance {tolerance}: runs back at frame {i}: {} → {}", pair[0].0, pair[1].0);
            }
        }
    }
}

/// Smoothing a curve the Smoother already smoothed changes nothing. (Keys it could not thin out
/// the first time, because the original path ran back, it made smooth; the smooth path can then
/// be thinned out: that second run's result stays as it is.)
#[test]
fn smoothing_smooth_keys_again_changes_nothing() {
    for (straight, tolerance) in [(true, 1.0), (false, 1.0), (false, 0.1), (true, 20.0), (false, 20.0)] {
        let (mut s, l) = setup();
        s.prefs.general.default_spatial_linear = straight;
        animate(&mut s, l, "transform/position", &LINE.map(|(t, v)| (t, json!(v))));
        let mut runs = Vec::new();
        for _ in 0..3 {
            select(&mut s, l, "transform/position");
            s.execute("keys.smooth", json!({"tolerance": tolerance})).unwrap();
            runs.push(prop(&s, l, "transform/position").keys);
        }
        assert_eq!(runs[2], runs[1], "straight {straight}, tolerance {tolerance}: a third run changes nothing");
        if runs[0].len() < LINE.len() {
            assert_eq!(runs[1], runs[0], "straight {straight}, tolerance {tolerance}: thinned out once, a second run changes nothing");
        }
        assert!(runs[1].len() <= runs[0].len());
    }
}

#[test]
fn motion_sketch_records_position_keys() {
    let (mut s, l) = setup();
    // 1 s of capture: a straight line from (0,0) to (300,150), sampled every 0.1 s.
    let pts: Vec<serde_json::Value> = (0..=10).map(|i| json!([i as f64 * 0.1, i as f64 * 30.0, i as f64 * 15.0])).collect();
    let r = s.execute("motion.sketch", json!({"layer": l, "points": pts, "start": 0.5, "smoothing": 0})).unwrap();
    let p = prop(&s, l, "transform/position");
    assert_eq!(r["keys"], 31);
    assert_eq!(p.keys.len(), 31);
    assert_eq!(p.keys[0].time.seconds(), 0.5);
    assert_eq!(p.keys[0].value.as_vec2(), [0.0, 0.0]);
    let last = p.keys[30].value.as_vec2();
    assert!((last[0] - 300.0).abs() < 1e-6 && (last[1] - 150.0).abs() < 1e-6);
    // Smoothing collapses the straight line to (almost) its end points; 200 % capture speed
    // plays back over twice the time.
    s.execute("edit.undo", json!({})).unwrap();
    s.execute("motion.sketch", json!({"layer": l, "points": pts, "start": 0, "smoothing": 2, "captureSpeed": 200})).unwrap();
    let p = prop(&s, l, "transform/position");
    assert!(p.keys.len() <= 4, "{}", p.keys.len());
    assert!((p.keys.last().unwrap().time.seconds() - 2.0).abs() < 1e-9);
    // [x, y] points: one per frame.
    s.execute("motion.sketch", json!({"layer": l, "points": [[0, 0], [10, 0], [20, 0]], "start": 3, "smoothing": 0})).unwrap();
    let p = prop(&s, l, "transform/position");
    let t = s.active_comp().unwrap().frame_rate.tick_of(92);
    assert!(p.keys.iter().any(|k| k.time == t && k.value.as_vec2() == [20.0, 0.0]));
    assert!(s.execute("motion.sketch", json!({"layer": l, "points": [[1, 2]]})).is_err());
}
