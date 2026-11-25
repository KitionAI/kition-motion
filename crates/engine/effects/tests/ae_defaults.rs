//! Effects start at the defaults of the kit's target app. Blurs, Sharpen and Noise start
//! at 0, where they change nothing; an agent gives the amount when it applies them
//! (`effect.apply {params}`, see agent/guide.md).
#![allow(clippy::unwrap_used)]

use kition_motion_keyframe::Value;

#[test]
fn blurs_sharpen_and_noise_start_at_after_effects_defaults() {
    for (effect, param) in
        [("Gaussian Blur", "blurriness"), ("Fast Box Blur", "radius"), ("Directional Blur", "length"), ("Sharpen", "amount"), ("Noise", "amount")]
    {
        let spec = kition_motion_effects::lookup(effect).unwrap();
        let default = spec.params.iter().find(|p| p.id == param).unwrap();
        assert_eq!(kition_motion_effects::default_value(default, [100.0, 100.0]), Value::Scalar(0.0), "{effect} {param}");
    }
}
