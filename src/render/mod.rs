mod element;
mod transform;

pub use element::ImageElement;
pub use transform::{FitMode, ViewTransform};

use std::sync::Arc;

use fast_image_resize::{self as fr, images::Image as FrImage, images::ImageRef};
use gpui::RenderImage;
use image::{Delay, Frame, RgbaImage};

use crate::decode::{Decoded, DecodedImage, Orientation};

pub fn to_bgra(image: &DecodedImage) -> RgbaImage {
    let mut buffer = image.rgba.clone();
    for px in buffer.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    RgbaImage::from_raw(image.width, image.height, buffer)
        .expect("decoded buffer must match its dimensions")
}

pub fn to_render_image_still(image: &DecodedImage) -> Arc<RenderImage> {
    Arc::new(RenderImage::new(vec![Frame::new(to_bgra(image))]))
}

pub fn into_render_image_still(image: DecodedImage) -> Arc<RenderImage> {
    Arc::new(RenderImage::new(vec![Frame::new(into_bgra(image))]))
}

fn into_bgra(image: DecodedImage) -> RgbaImage {
    let mut buffer = image.rgba;
    for px in buffer.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    RgbaImage::from_raw(image.width, image.height, buffer)
        .expect("decoded buffer must match its dimensions")
}

pub fn to_render_image(decoded: &Decoded) -> Arc<RenderImage> {
    let frames: Vec<Frame> = match decoded {
        Decoded::Still(image) => vec![Frame::new(to_bgra(image))],
        Decoded::Animation(frames) => frames
            .iter()
            .map(|f| {
                Frame::from_parts(to_bgra(&f.image), 0, 0, Delay::from_saturating_duration(f.delay))
            })
            .collect(),
    };
    Arc::new(RenderImage::new(frames))
}

pub const DOWNSCALE_SLACK: f32 = 1.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resample {
    Filtered,
    Nearest,
}

impl Resample {
    pub fn from_antialias(antialias: bool) -> Self {
        if antialias { Resample::Filtered } else { Resample::Nearest }
    }

    fn algorithm(self) -> fr::ResizeAlg {
        match self {
            Resample::Filtered => fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3),
            Resample::Nearest => fr::ResizeAlg::Nearest,
        }
    }
}

pub fn needs_downscale(image: (u32, u32), display: (f32, f32)) -> bool {
    display.0 > 0.0
        && display.1 > 0.0
        && (image.0 as f32 > display.0 * DOWNSCALE_SLACK
            || image.1 as f32 > display.1 * DOWNSCALE_SLACK)
}

pub fn downscale_to_display(image: &DecodedImage, display: (f32, f32)) -> DecodedImage {
    downscale_to_display_with(image, display, Resample::Filtered)
}

pub fn downscaled<'a>(
    image: &'a DecodedImage,
    display: (f32, f32),
    resample: Resample,
) -> std::borrow::Cow<'a, DecodedImage> {
    if needs_downscale((image.width, image.height), display) {
        std::borrow::Cow::Owned(downscale_to_display_with(image, display, resample))
    } else {
        std::borrow::Cow::Borrowed(image)
    }
}

pub fn downscale_to_display_with(
    image: &DecodedImage,
    display: (f32, f32),
    resample: Resample,
) -> DecodedImage {
    if !needs_downscale((image.width, image.height), display) {
        return image.clone();
    }
    let scale = (display.0 / image.width as f32).max(display.1 / image.height as f32);
    let width = ((image.width as f32 * scale).round() as u32).max(1);
    let height = ((image.height as f32 * scale).round() as u32).max(1);

    resize_rgba(image, width, height, resample).unwrap_or_else(|_| DecodedImage {
        width,
        height,
        rgba: vec![0; (width * height * 4) as usize],
    })
}

fn resize_rgba(
    image: &DecodedImage,
    width: u32,
    height: u32,
    resample: Resample,
) -> Result<DecodedImage, Box<dyn std::error::Error>> {
    let source = ImageRef::new(image.width, image.height, &image.rgba, fr::PixelType::U8x4)?;
    let mut destination = FrImage::new(width, height, fr::PixelType::U8x4);
    let options = fr::ResizeOptions::new().resize_alg(resample.algorithm());
    fr::Resizer::new().resize(&source, &mut destination, &options)?;
    Ok(DecodedImage { width, height, rgba: destination.into_vec() })
}

pub const MAX_MAGNIFY_FACTOR: u32 = 64;

pub fn magnify_factor(zoom: f32) -> u32 {
    if !zoom.is_finite() || zoom <= 1.0 {
        return 1;
    }
    (zoom.ceil() as u32).clamp(1, MAX_MAGNIFY_FACTOR)
}

pub const MAGNIFY_BUDGET_BYTES: u64 = 96 * 1024 * 1024;

pub fn fit_factor_to_budget(crop: (u32, u32, u32, u32), factor: u32) -> u32 {
    let px = (crop.2 as u64) * (crop.3 as u64);
    if px == 0 {
        return 1;
    }
    let mut f = factor.max(1);
    while f > 1 && px * (f as u64) * (f as u64) * 4 > MAGNIFY_BUDGET_BYTES {
        f -= 1;
    }
    f
}

pub fn magnify_nearest(image: &DecodedImage, factor: u32) -> DecodedImage {
    magnify_nearest_crop(image, (0, 0, image.width, image.height), factor)
}

pub fn magnify_nearest_crop(
    image: &DecodedImage,
    crop: (u32, u32, u32, u32),
    factor: u32,
) -> DecodedImage {
    let (cx, cy, cw, ch) = crop;
    let cx = cx.min(image.width) as usize;
    let cy = cy.min(image.height) as usize;
    let cw = (cw as usize).min(image.width as usize - cx);
    let ch = (ch as usize).min(image.height as usize - cy);
    let f = factor.max(1) as usize;

    if cw == 0 || ch == 0 {
        return DecodedImage { width: 0, height: 0, rgba: Vec::new() };
    }
    if f == 1 && cx == 0 && cy == 0 && cw == image.width as usize && ch == image.height as usize {
        return image.clone();
    }

    let src_w = image.width as usize;
    let out_w = cw * f;
    let out_h = ch * f;

    let row_bytes = out_w * 4;
    let mut out = vec![0u8; row_bytes * out_h];

    for y in 0..ch {
        let src_row = &image.rgba[((cy + y) * src_w + cx) * 4..][..cw * 4];
        let (first, rest) = out[y * f * row_bytes..].split_at_mut(row_bytes);
        for (x, px) in src_row.chunks_exact(4).enumerate() {
            let block = &mut first[x * f * 4..(x * f + f) * 4];
            block[..4].copy_from_slice(px);
            let mut done = 4;
            while done < block.len() {
                let take = done.min(block.len() - done);
                block.copy_within(0..take, done);
                done += take;
            }
        }
        for j in 0..f.saturating_sub(1) {
            rest[j * row_bytes..j * row_bytes + row_bytes].copy_from_slice(first);
        }
    }

    DecodedImage { width: out_w as u32, height: out_h as u32, rgba: out }
}

pub fn oriented(
    image: &DecodedImage,
    orientation: Orientation,
) -> std::borrow::Cow<'_, DecodedImage> {
    if orientation == Orientation::Normal {
        std::borrow::Cow::Borrowed(image)
    } else {
        std::borrow::Cow::Owned(apply_orientation(image, orientation))
    }
}

pub fn apply_orientation(image: &DecodedImage, orientation: Orientation) -> DecodedImage {
    if orientation == Orientation::Normal {
        return image.clone();
    }
    let (w, h) = (image.width as usize, image.height as usize);
    let (out_w, out_h) = match orientation {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Transpose
        | Orientation::Transverse => (h, w),
        _ => (w, h),
    };

    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = match orientation {
                Orientation::FlipH => (w - 1 - x, y),
                Orientation::Rotate180 => (w - 1 - x, h - 1 - y),
                Orientation::FlipV => (x, h - 1 - y),
                Orientation::Transpose => (y, x),
                Orientation::Rotate90 => (h - 1 - y, x),
                Orientation::Transverse => (h - 1 - y, w - 1 - x),
                Orientation::Rotate270 => (y, w - 1 - x),
                Orientation::Normal => (x, y),
            };
            let src = (y * w + x) * 4;
            let dst = (ny * out_w + nx) * 4;
            out[dst..dst + 4].copy_from_slice(&image.rgba[src..src + 4]);
        }
    }

    DecodedImage { rgba: out, width: out_w as u32, height: out_h as u32 }
}
