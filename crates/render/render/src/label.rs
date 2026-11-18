//! A short label burned into a frame (a contact sheet's frame times): light text on a dark box in
//! the frame's bottom-left corner, sized to the frame, in the bundled default face, so it draws
//! the same everywhere.

use kition_motion_geom::{Mat3, vec2};
use kition_motion_keyframe::TextDoc;
use kition_motion_path::FillRule;
use kition_motion_raster::Image;
use kurbo::{Affine, BezPath};

/// The box behind the text: black at this opacity.
const BOX_ALPHA: f32 = 0.72;

/// `text`'s glyph outlines at `size` pixels, origin on the first baseline.
fn outlines(text: &str, size: f64) -> Vec<BezPath> {
    let doc = TextDoc { font: kition_motion_text::fonts::DEFAULT_FAMILY.into(), style: "Bold".into(), size, ..TextDoc::plain(text) };
    kition_motion_text::layout_doc(&doc).glyphs.iter().filter(|g| !g.is_space).map(|g| Affine::translate(g.origin.to_vec2()) * g.path.clone()).collect()
}

/// Paints `coverage` (0–1 per pixel of `img`) in premultiplied `colour` at `opacity` over `img`.
fn paint(img: &mut Image, coverage: &[f32], colour: [f32; 3], opacity: f32) {
    for (px, cover) in img.data.iter_mut().zip(coverage) {
        let a = (cover * opacity).clamp(0.0, 1.0);
        if a > 0.0 {
            for (c, value) in colour.iter().enumerate() {
                px[c] = px[c] * (1.0 - a) + value * a;
            }
            px[3] = px[3] * (1.0 - a) + a;
        }
    }
}

/// Burns `text` into `img`'s bottom-left corner. A frame too small for legible text (under 24
/// pixels on a side) is left as it is.
pub fn burn_label(img: &mut Image, text: &str) {
    if img.height < 24 || img.width < 24 || text.trim().is_empty() {
        return;
    }
    let (w, h) = (f64::from(img.width), f64::from(img.height));
    let margin = (h * 0.04).max(2.0);
    let inner = (h * 0.03).max(2.0);
    let mut size = (h * 0.12).clamp(9.0, 48.0);
    let mut paths = outlines(text, size);
    let Some(mut bounds) = kition_motion_path::bounds(&paths) else { return };
    let room = w - 2.0 * (margin + inner);
    if bounds.width() > room && bounds.width() > 0.0 {
        size *= room / bounds.width();
        paths = outlines(text, size);
        let Some(fitted) = kition_motion_path::bounds(&paths) else { return };
        bounds = fitted;
    }
    let (tw, th) = (bounds.width(), bounds.height());
    let (x1, y1) = ((margin + 2.0 * inner + tw).min(w), h - margin);
    let (x0, y0) = (margin, (y1 - th - 2.0 * inner).max(0.0));
    let backing = kition_motion_path::rect([x1 - x0, y1 - y0], [(x0 + x1) / 2.0, (y0 + y1) / 2.0], 0.0);
    let boxed = kition_motion_path::fill_coverage(&[backing], &Mat3::IDENTITY, img.width, img.height, FillRule::NonZero);
    paint(img, &boxed.data, [0.0; 3], BOX_ALPHA);
    let at = Mat3::translate(vec2((x0 + inner - bounds.x0).round(), (y0 + inner - bounds.y0).round()));
    let glyphs = kition_motion_path::fill_coverage(&paths, &at, img.width, img.height, FillRule::NonZero);
    paint(img, &glyphs.data, [1.0; 3], 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burns_light_text_on_a_dark_box_in_the_bottom_left_corner() {
        let mut img = Image::filled(320, 180, [1.0, 0.0, 0.0, 1.0]);
        burn_label(&mut img, "1.5 s");
        let px = |x: u32, y: u32| img.data[img.idx(x, y)];
        assert_eq!(px(300, 20), [1.0, 0.0, 0.0, 1.0], "the rest of the frame is untouched");
        let corner: Vec<[f32; 4]> = (0..80).flat_map(|x| (130..175).map(move |y| (x, y))).map(|(x, y)| px(x, y)).collect();
        assert!(corner.iter().any(|p| p[0] < 0.4 && p[1] < 0.1), "a dark box");
        assert!(corner.iter().filter(|p| p.iter().all(|c| *c > 0.9)).count() > 10, "white text");
        let mut small = Image::filled(20, 20, [0.0, 0.0, 1.0, 1.0]);
        burn_label(&mut small, "1 s");
        assert_eq!(small, Image::filled(20, 20, [0.0, 0.0, 1.0, 1.0]), "too small for text");
    }
}
