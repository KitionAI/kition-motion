//! A new shape takes its gradient's colours and direction in the same step (video plan P0): every
//! gradient a shape could be made with ran white to black, left to right.
use kition_motion_engine::Session;
use kition_motion_time::Tick;
use serde_json::json;

/// A 400×200 comp with one rectangle filling it, made with `paint`; the RGBA pixel at `at`.
fn shape(paint: serde_json::Value) -> (Session, impl Fn(&Session, [u32; 2]) -> [u8; 4]) {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Gradient", "width": 400, "height": 200, "duration": 1, "frameRate": 30})).unwrap();
    let mut p = json!({"kind": "rect", "size": [400, 200], "position": [200, 100]});
    p.as_object_mut().unwrap().extend(paint.as_object().unwrap().clone());
    s.execute("layer.newShape", p).unwrap();
    let pixel = |s: &Session, [x, y]: [u32; 2]| {
        let (w, _, px) = s.render_rgba8(s.active_comp_id().unwrap(), Tick::ZERO, 0).unwrap();
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    (s, pixel)
}

#[test]
fn a_shape_fill_takes_its_gradient_colours() {
    let (s, pixel) = shape(json!({"fillGradient": ["#ff0000", "#0000ff"]}));
    let (left, right) = (pixel(&s, [2, 100]), pixel(&s, [397, 100]));
    assert!(left[0] > 200 && left[2] < 50, "left {left:?}");
    assert!(right[2] > 200 && right[0] < 50, "right {right:?}");
}

#[test]
fn a_shape_fill_gradient_runs_at_its_angle() {
    let (s, pixel) = shape(json!({"fillGradient": ["#ff0000", "#0000ff"], "fillAngle": 90}));
    let (top, bottom) = (pixel(&s, [200, 2]), pixel(&s, [200, 197]));
    assert!(top[0] > 200 && top[2] < 50, "top {top:?}");
    assert!(bottom[2] > 200 && bottom[0] < 50, "bottom {bottom:?}");
    // Across a row the colour stays the same.
    let (left, right) = (pixel(&s, [2, 100]), pixel(&s, [397, 100]));
    assert!(left.iter().zip(right).all(|(a, b)| a.abs_diff(b) <= 2), "{left:?} {right:?}");
}

#[test]
fn a_bad_gradient_is_refused_and_names_the_forms() {
    let mut s = Session::default();
    s.execute("comp.new", json!({"name": "Gradient", "width": 40, "height": 20, "duration": 1})).unwrap();
    let e = s.execute("layer.newShape", json!({"kind": "rect", "fillGradient": ["#ffffff"]})).unwrap_err().to_string();
    assert!(e.contains("fillGradient") && e.contains("two or more colours"), "{e}");
}
