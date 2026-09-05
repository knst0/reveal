use std::ffi::c_void;
use std::ptr::NonNull;
use std::time::Duration;

use rav1d::include::dav1d::dav1d::Dav1dContext;
use rav1d::include::dav1d::headers::{
    DAV1D_MC_BT709, DAV1D_MC_IDENTITY, DAV1D_PIXEL_LAYOUT_I400, DAV1D_PIXEL_LAYOUT_I420,
    DAV1D_PIXEL_LAYOUT_I422, DAV1D_PIXEL_LAYOUT_I444, Dav1dPixelLayout,
};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{
    dav1d_close, dav1d_data_wrap, dav1d_default_settings, dav1d_get_picture, dav1d_open,
    dav1d_picture_unref, dav1d_send_data,
};
use zenavif_parse::AvifParser;

use super::{DecodeError, DecodeRequest, Decoded, DecodedImage, Decoder, Frame, extension_of};

pub struct AvifDecoder;

pub const AVIF_EXTENSIONS: &[&str] = &["avif", "avifs"];

pub fn is_avif_extension(ext: &str) -> bool {
    matches!(ext, "avif" | "avifs")
}

fn looks_like_avif(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && matches!(&bytes[8..12], b"avif" | b"avis")
}

fn err(msg: &str) -> DecodeError {
    DecodeError::Decode(msg.into())
}

struct Plane {
    data: Vec<u8>,
    width: usize,
    height: usize,
}

struct YuvPicture {
    width: usize,
    height: usize,
    planes: Vec<Plane>,
    monochrome: bool,
    full_range: bool,
    bt709: bool,
}

struct Av1Decoder {
    ctx: Option<Dav1dContext>,
}

unsafe extern "C" fn free_boxed_slice(
    _buf: *const u8,
    cookie: Option<rav1d::src::send_sync_non_null::SendSyncNonNull<c_void>>,
) {
    if let Some(cookie) = cookie {
        drop(unsafe { Box::from_raw(cookie.as_ptr().as_ptr() as *mut Vec<u8>) });
    }
}

impl Av1Decoder {
    fn new() -> Result<Self, DecodeError> {
        let mut settings = std::mem::MaybeUninit::uninit();
        let settings = unsafe {
            dav1d_default_settings(NonNull::new(settings.as_mut_ptr()).unwrap());
            settings.assume_init()
        };
        let mut settings = settings;
        let mut ctx: Option<Dav1dContext> = None;
        let res = unsafe { dav1d_open(NonNull::new(&mut ctx), NonNull::new(&mut settings)) };
        if res.0 != 0 {
            return Err(err("failed to open the av1 decoder"));
        }
        if ctx.is_none() {
            return Err(err("the av1 decoder returned no context"));
        }
        Ok(Self { ctx })
    }

    fn decode_all(&mut self, payload: &[u8]) -> Result<Vec<YuvPicture>, DecodeError> {
        let ctx = self.ctx.ok_or_else(|| err("the av1 decoder is closed"))?;

        let owned: Box<Vec<u8>> = Box::new(payload.to_vec());
        let len = owned.len();
        let ptr = owned.as_ptr() as *mut u8;
        let cookie = Box::into_raw(owned) as *mut c_void;

        let mut data = rav1d::include::dav1d::data::Dav1dData::default();
        let res = unsafe {
            dav1d_data_wrap(
                NonNull::new(&mut data),
                NonNull::new(ptr),
                len,
                Some(free_boxed_slice),
                Some(rav1d::src::send_sync_non_null::SendSyncNonNull::new_unchecked(
                    NonNull::new(cookie).unwrap(),
                )),
            )
        };
        if res.0 != 0 {
            drop(unsafe { Box::from_raw(cookie as *mut Vec<u8>) });
            return Err(err("failed to hand the av1 payload to the decoder"));
        }

        let mut data = DataGuard(data);
        let mut out = Vec::new();
        loop {
            while data.0.sz > 0 {
                let res = unsafe { dav1d_send_data(Some(ctx), NonNull::new(&mut data.0)) };
                if res.0 == 0 {
                    continue;
                }
                if is_again(res.0) {
                    break;
                }
                return Err(err("the av1 decoder rejected the payload"));
            }

            match self.take_picture(ctx)? {
                Some(pic) => out.push(pic),
                None if data.0.sz == 0 => break,
                None => {}
            }
        }

        while let Some(pic) = self.take_picture(ctx)? {
            out.push(pic);
        }

        if out.is_empty() {
            return Err(err("the av1 stream contained no pictures"));
        }
        Ok(out)
    }

    fn take_picture(&mut self, ctx: Dav1dContext) -> Result<Option<YuvPicture>, DecodeError> {
        let mut pic = Dav1dPicture::default();
        let res = unsafe { dav1d_get_picture(Some(ctx), NonNull::new(&mut pic)) };
        if is_again(res.0) {
            return Ok(None);
        }
        if res.0 != 0 {
            return Ok(None);
        }
        let guard = PictureGuard(&mut pic);
        let converted = copy_picture(guard.0);
        drop(guard);
        converted.map(Some)
    }
}

impl Drop for Av1Decoder {
    fn drop(&mut self) {
        if self.ctx.is_some() {
            unsafe { dav1d_close(NonNull::new(&mut self.ctx)) };
        }
    }
}

struct DataGuard(rav1d::include::dav1d::data::Dav1dData);

impl Drop for DataGuard {
    fn drop(&mut self) {
        unsafe { rav1d::src::lib::dav1d_data_unref(NonNull::new(&mut self.0)) };
    }
}

struct PictureGuard<'a>(&'a mut Dav1dPicture);

impl Drop for PictureGuard<'_> {
    fn drop(&mut self) {
        unsafe { dav1d_picture_unref(NonNull::new(self.0 as *mut Dav1dPicture)) };
    }
}

fn is_again(code: i32) -> bool {
    -code == libc_eagain()
}

fn libc_eagain() -> i32 {
    11
}

fn plane_dims(layout: Dav1dPixelLayout, w: usize, h: usize) -> (usize, usize) {
    if layout == DAV1D_PIXEL_LAYOUT_I420 {
        (w.div_ceil(2), h.div_ceil(2))
    } else if layout == DAV1D_PIXEL_LAYOUT_I422 {
        (w.div_ceil(2), h)
    } else {
        (w, h)
    }
}

fn read_plane(
    base: *const u8,
    stride: isize,
    width: usize,
    height: usize,
    bpc: u32,
) -> Result<Plane, DecodeError> {
    if base.is_null() {
        return Err(err("the av1 decoder produced an empty plane"));
    }
    let shift = bpc.saturating_sub(8);
    let mut data = Vec::with_capacity(width * height);
    for y in 0..height {
        let row = unsafe { base.offset(stride * y as isize) };
        if bpc <= 8 {
            let row = unsafe { std::slice::from_raw_parts(row, width) };
            data.extend_from_slice(row);
        } else {
            let row = unsafe { std::slice::from_raw_parts(row as *const u16, width) };
            data.extend(row.iter().map(|&v| (v >> shift).min(255) as u8));
        }
    }
    Ok(Plane { data, width, height })
}

fn copy_picture(pic: &Dav1dPicture) -> Result<YuvPicture, DecodeError> {
    let width = usize::try_from(pic.p.w).map_err(|_| err("the av1 picture has a bad width"))?;
    let height = usize::try_from(pic.p.h).map_err(|_| err("the av1 picture has a bad height"))?;
    if width == 0 || height == 0 {
        return Err(err("the av1 picture is empty"));
    }
    let bpc = u32::try_from(pic.p.bpc).map_err(|_| err("the av1 picture has a bad bit depth"))?;
    if !matches!(bpc, 8 | 10 | 12) {
        return Err(err("the av1 picture has an unsupported bit depth"));
    }

    let layout = pic.p.layout;
    let monochrome = layout == DAV1D_PIXEL_LAYOUT_I400;
    if !monochrome
        && layout != DAV1D_PIXEL_LAYOUT_I420
        && layout != DAV1D_PIXEL_LAYOUT_I422
        && layout != DAV1D_PIXEL_LAYOUT_I444
    {
        return Err(err("the av1 picture has an unsupported pixel layout"));
    }

    let luma = pic.data[0].ok_or_else(|| err("the av1 picture has no luma plane"))?;
    let mut planes =
        vec![read_plane(luma.as_ptr() as *const u8, pic.stride[0] as isize, width, height, bpc)?];

    if !monochrome {
        let (cw, ch) = plane_dims(layout, width, height);
        for i in 1..3 {
            let p = pic.data[i].ok_or_else(|| err("the av1 picture has no chroma plane"))?;
            planes.push(read_plane(p.as_ptr() as *const u8, pic.stride[1] as isize, cw, ch, bpc)?);
        }
    }

    let (mut full_range, mut bt709) = (false, false);
    if let Some(seq) = pic.seq_hdr {
        let seq = unsafe { seq.as_ref() };
        full_range = seq.color_range != 0;
        bt709 = seq.mtrx == DAV1D_MC_BT709;
        if seq.mtrx == DAV1D_MC_IDENTITY {
            full_range = true;
        }
    }

    Ok(YuvPicture { width, height, planes, monochrome, full_range, bt709 })
}

fn yuv_to_rgba(pic: &YuvPicture, alpha: Option<&Plane>) -> DecodedImage {
    let (w, h) = (pic.width, pic.height);
    let mut rgba = Vec::with_capacity(w * h * 4);

    let (kr, kb) = if pic.bt709 { (0.2126f32, 0.0722f32) } else { (0.299f32, 0.114f32) };
    let kg = 1.0 - kr - kb;

    let y_plane = &pic.planes[0];
    for y in 0..h {
        for x in 0..w {
            let yv = f32::from(y_plane.data[y * y_plane.width + x]);
            let (yn, u, v) = if pic.monochrome {
                let yn = if pic.full_range { yv } else { (yv - 16.0) * (255.0 / 219.0) };
                (yn, 0.0, 0.0)
            } else {
                let cu = &pic.planes[1];
                let cv = &pic.planes[2];
                let cx = (x * cu.width / w).min(cu.width.saturating_sub(1));
                let cy = (y * cu.height / h).min(cu.height.saturating_sub(1));
                let u = f32::from(cu.data[cy * cu.width + cx]) - 128.0;
                let v = f32::from(cv.data[cy * cv.width + cx]) - 128.0;
                let (yn, u, v) = if pic.full_range {
                    (yv, u, v)
                } else {
                    ((yv - 16.0) * (255.0 / 219.0), u * (255.0 / 224.0), v * (255.0 / 224.0))
                };
                (yn, u, v)
            };

            let r = yn + 2.0 * (1.0 - kr) * v;
            let b = yn + 2.0 * (1.0 - kb) * u;
            let g = yn - (2.0 * (1.0 - kr) * kr / kg) * v - (2.0 * (1.0 - kb) * kb / kg) * u;

            let a = alpha
                .map(|p| {
                    let ax = (x * p.width / w).min(p.width.saturating_sub(1));
                    let ay = (y * p.height / h).min(p.height.saturating_sub(1));
                    p.data[ay * p.width + ax]
                })
                .unwrap_or(255);

            rgba.extend_from_slice(&[clamp8(r), clamp8(g), clamp8(b), a]);
        }
    }

    DecodedImage { rgba, width: w as u32, height: h as u32 }
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn unpremultiply(image: &mut DecodedImage) {
    for px in image.rgba.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a == 0 || a == 255 {
            continue;
        }
        for c in &mut px[..3] {
            *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
        }
    }
}

fn decode_one(payload: &[u8]) -> Result<YuvPicture, DecodeError> {
    let mut decoder = Av1Decoder::new()?;
    let mut pics = decoder.decode_all(payload)?;
    Ok(pics.swap_remove(0))
}

fn decode_alpha(payload: &[u8]) -> Option<Plane> {
    let pic = decode_one(payload).ok()?;
    let mut pic = pic;
    let mut plane = pic.planes.swap_remove(0);
    if !pic.full_range {
        for v in &mut plane.data {
            let scaled = (f32::from(*v) - 16.0) * (255.0 / 219.0);
            *v = clamp8(scaled);
        }
    }
    Some(plane)
}

fn image_from(
    payload: &[u8],
    alpha: Option<&[u8]>,
    premultiplied: bool,
) -> Result<DecodedImage, DecodeError> {
    let pic = decode_one(payload)?;
    let alpha = alpha.and_then(decode_alpha);
    let mut image = yuv_to_rgba(&pic, alpha.as_ref());
    if premultiplied && alpha.is_some() {
        unpremultiply(&mut image);
    }
    Ok(image)
}

fn assemble_grid(
    parser: &AvifParser<'_>,
    grid: &zenavif_parse::GridConfig,
) -> Result<DecodedImage, DecodeError> {
    let (rows, columns) = (usize::from(grid.rows), usize::from(grid.columns));
    let count = rows * columns;
    if count == 0 || parser.grid_tile_count() < count {
        return Err(err("the avif grid is missing tiles"));
    }

    let mut tiles = Vec::with_capacity(count);
    for index in 0..count {
        let data = parser.tile_data(index).map_err(|e| DecodeError::Decode(e.to_string()))?;
        let pic = decode_one(&data)?;
        tiles.push(yuv_to_rgba(&pic, None));
    }

    let (tw, th) = (tiles[0].width as usize, tiles[0].height as usize);
    if tiles.iter().any(|t| t.width as usize != tw || t.height as usize != th) {
        return Err(err("the avif grid tiles have mismatched sizes"));
    }

    let full_w = tw * columns;
    let full_h = th * rows;
    let out_w =
        if grid.output_width == 0 { full_w } else { (grid.output_width as usize).min(full_w) };
    let out_h =
        if grid.output_height == 0 { full_h } else { (grid.output_height as usize).min(full_h) };
    if out_w == 0 || out_h == 0 {
        return Err(err("the avif grid has empty output dimensions"));
    }

    let mut rgba = vec![0u8; out_w * out_h * 4];
    for y in 0..out_h {
        let (row, ty) = (y / th, y % th);
        for column in 0..columns {
            let tile = &tiles[row * columns + column];
            let x0 = column * tw;
            if x0 >= out_w {
                break;
            }
            let span = tw.min(out_w - x0);
            let src = (ty * tw) * 4;
            let dst = (y * out_w + x0) * 4;
            rgba[dst..dst + span * 4].copy_from_slice(&tile.rgba[src..src + span * 4]);
        }
    }

    Ok(DecodedImage { rgba, width: out_w as u32, height: out_h as u32 })
}

impl Decoder for AvifDecoder {
    fn name(&self) -> &'static str {
        "avif"
    }

    fn probe(&self, req: &DecodeRequest<'_>) -> bool {
        looks_like_avif(req.bytes) || extension_of(req.path).is_some_and(|e| is_avif_extension(&e))
    }

    fn decode(&self, req: &DecodeRequest<'_>) -> Result<Decoded, DecodeError> {
        let parser =
            AvifParser::from_bytes(req.bytes).map_err(|e| DecodeError::Decode(e.to_string()))?;
        let premultiplied = parser.premultiplied_alpha();

        if let Some(info) = parser.animation_info()
            && info.frame_count > 1
        {
            let mut frames = Vec::with_capacity(info.frame_count);
            for index in 0..info.frame_count {
                let frame = parser.frame(index).map_err(|e| DecodeError::Decode(e.to_string()))?;
                let image = image_from(&frame.data, frame.alpha_data.as_deref(), premultiplied)?;
                let ms = u64::from(frame.duration_ms.max(1));
                frames.push(Frame { image, delay: Duration::from_millis(ms) });
            }
            if !frames.is_empty() {
                return Ok(Decoded::Animation(frames));
            }
        }

        if let Some(grid) = parser.grid_config()
            && parser.grid_tile_count() > 0
            && let Ok(image) = assemble_grid(&parser, grid)
        {
            return Ok(Decoded::Still(image));
        }

        let primary = parser.primary_data().map_err(|e| DecodeError::Decode(e.to_string()))?;
        let alpha = match parser.alpha_data() {
            Some(Ok(a)) => Some(a),
            _ => None,
        };
        let image = image_from(&primary, alpha.as_deref(), premultiplied)?;
        Ok(Decoded::Still(image))
    }
}

#[cfg(test)]
mod tests {
    use super::{AVIF_EXTENSIONS, is_avif_extension};

    #[test]
    fn every_listed_avif_extension_is_recognised() {
        for ext in AVIF_EXTENSIONS {
            assert!(is_avif_extension(ext), "{ext} is listed but not recognised");
        }
    }
}
