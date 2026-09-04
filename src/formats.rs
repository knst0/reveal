pub struct FormatInfo {
    pub extension: &'static str,
    pub mime: &'static str,
    pub description: &'static str,
}

pub const FORMATS: &[FormatInfo] = &[
    FormatInfo { extension: "png", mime: "image/png", description: "PNG image" },
    FormatInfo { extension: "jpg", mime: "image/jpeg", description: "JPEG image" },
    FormatInfo { extension: "jpeg", mime: "image/jpeg", description: "JPEG image" },
    FormatInfo { extension: "gif", mime: "image/gif", description: "GIF image" },
    FormatInfo { extension: "webp", mime: "image/webp", description: "WebP image" },
    FormatInfo { extension: "bmp", mime: "image/bmp", description: "Bitmap image" },
    FormatInfo { extension: "tif", mime: "image/tiff", description: "TIFF image" },
    FormatInfo { extension: "tiff", mime: "image/tiff", description: "TIFF image" },
    FormatInfo { extension: "ico", mime: "image/x-icon", description: "Icon" },
    FormatInfo { extension: "tga", mime: "image/x-tga", description: "TGA image" },
    FormatInfo { extension: "pnm", mime: "image/x-portable-anymap", description: "PNM image" },
    FormatInfo { extension: "hdr", mime: "image/vnd.radiance", description: "Radiance HDR" },
    FormatInfo { extension: "avif", mime: "image/avif", description: "AVIF image" },
    FormatInfo { extension: "svg", mime: "image/svg+xml", description: "SVG image" },
    FormatInfo { extension: "jxl", mime: "image/jxl", description: "JPEG XL image" },
];

pub fn mime_list() -> Vec<&'static str> {
    let mut mimes: Vec<&'static str> = FORMATS.iter().map(|f| f.mime).collect();
    mimes.sort_unstable();
    mimes.dedup();
    mimes
}

pub fn extensions() -> Vec<&'static str> {
    FORMATS.iter().map(|f| f.extension).collect()
}
