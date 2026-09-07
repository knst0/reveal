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
    FormatInfo { extension: "pbm", mime: "image/x-portable-bitmap", description: "PBM image" },
    FormatInfo { extension: "pgm", mime: "image/x-portable-graymap", description: "PGM image" },
    FormatInfo { extension: "ppm", mime: "image/x-portable-pixmap", description: "PPM image" },
    FormatInfo {
        extension: "pam",
        mime: "image/x-portable-arbitrarymap",
        description: "PAM image",
    },
    FormatInfo { extension: "ff", mime: "image/x-farbfeld", description: "farbfeld image" },
    FormatInfo { extension: "qoi", mime: "image/qoi", description: "QOI image" },
    FormatInfo { extension: "exr", mime: "image/x-exr", description: "OpenEXR image" },
    FormatInfo { extension: "hdr", mime: "image/vnd.radiance", description: "Radiance HDR" },
    FormatInfo { extension: "avif", mime: "image/avif", description: "AVIF image" },
    FormatInfo { extension: "avifs", mime: "image/avif-sequence", description: "AVIF sequence" },
    FormatInfo { extension: "svg", mime: "image/svg+xml", description: "SVG image" },
    FormatInfo { extension: "svgz", mime: "image/svg+xml-compressed", description: "SVG image" },
    FormatInfo { extension: "jxl", mime: "image/jxl", description: "JPEG XL image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "cr2", mime: "image/x-canon-cr2", description: "Canon raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "cr3", mime: "image/x-canon-cr3", description: "Canon raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "crw", mime: "image/x-canon-crw", description: "Canon raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "nef", mime: "image/x-nikon-nef", description: "Nikon raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "nrw", mime: "image/x-nikon-nrw", description: "Nikon raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "arw", mime: "image/x-sony-arw", description: "Sony raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "srf", mime: "image/x-sony-srf", description: "Sony raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "sr2", mime: "image/x-sony-sr2", description: "Sony raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "orf", mime: "image/x-olympus-orf", description: "Olympus raw image" },
    #[cfg(feature = "raw")]
    FormatInfo {
        extension: "rw2",
        mime: "image/x-panasonic-rw2",
        description: "Panasonic raw image",
    },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "raf", mime: "image/x-fuji-raf", description: "Fujifilm raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "dng", mime: "image/x-adobe-dng", description: "Adobe DNG image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "pef", mime: "image/x-pentax-pef", description: "Pentax raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "erf", mime: "image/x-epson-erf", description: "Epson raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "kdc", mime: "image/x-kodak-kdc", description: "Kodak raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "dcr", mime: "image/x-kodak-dcr", description: "Kodak raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "mrw", mime: "image/x-minolta-mrw", description: "Minolta raw image" },
    #[cfg(feature = "raw")]
    FormatInfo {
        extension: "3fr",
        mime: "image/x-hasselblad-3fr",
        description: "Hasselblad raw image",
    },
    #[cfg(feature = "raw")]
    FormatInfo {
        extension: "iiq",
        mime: "image/x-phaseone-iiq",
        description: "Phase One raw image",
    },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "mos", mime: "image/x-leaf-mos", description: "Leaf raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "srw", mime: "image/x-samsung-srw", description: "Samsung raw image" },
    #[cfg(feature = "raw")]
    FormatInfo { extension: "x3f", mime: "image/x-sigma-x3f", description: "Sigma raw image" },
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

fn uti(mime: &str, extension: &str) -> String {
    match mime {
        "image/png" => "public.png".to_owned(),
        "image/jpeg" => "public.jpeg".to_owned(),
        "image/gif" => "com.compuserve.gif".to_owned(),
        "image/webp" => "org.webmproject.webp".to_owned(),
        "image/bmp" => "com.microsoft.bmp".to_owned(),
        "image/tiff" => "public.tiff".to_owned(),
        "image/x-icon" => "com.microsoft.ico".to_owned(),
        "image/x-tga" => "com.truevision.tga-image".to_owned(),
        "image/vnd.radiance" => "public.radiance".to_owned(),
        "image/avif" => "public.avif".to_owned(),
        "image/avif-sequence" => "public.avif".to_owned(),
        "image/svg+xml" => "public.svg-image".to_owned(),
        "image/jxl" => "public.jpeg-xl".to_owned(),
        _ => format!("io.github.knst0.reveal.{extension}"),
    }
}

pub fn info_plist(version: &str) -> String {
    let mut doc_types = String::new();
    for format in FORMATS {
        doc_types.push_str(&format!(
            "\t\t<dict>\n\
             \t\t\t<key>CFBundleTypeName</key>\n\t\t\t<string>{}</string>\n\
             \t\t\t<key>CFBundleTypeRole</key>\n\t\t\t<string>Viewer</string>\n\
             \t\t\t<key>LSHandlerRank</key>\n\t\t\t<string>Alternate</string>\n\
             \t\t\t<key>CFBundleTypeExtensions</key>\n\t\t\t<array>\n\t\t\t\t<string>{}</string>\n\t\t\t</array>\n\
             \t\t\t<key>LSItemContentTypes</key>\n\t\t\t<array>\n\t\t\t\t<string>{}</string>\n\t\t\t</array>\n\
             \t\t\t<key>CFBundleTypeIconFile</key>\n\t\t\t<string>reveal.icns</string>\n\
             \t\t</dict>\n",
            format.description,
            format.extension,
            uti(format.mime, format.extension),
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \t<key>CFBundleName</key>\n\t<string>Reveal</string>\n\
         \t<key>CFBundleDisplayName</key>\n\t<string>Reveal</string>\n\
         \t<key>CFBundleIdentifier</key>\n\t<string>io.github.knst0.reveal</string>\n\
         \t<key>CFBundleExecutable</key>\n\t<string>reveal</string>\n\
         \t<key>CFBundleIconFile</key>\n\t<string>reveal.icns</string>\n\
         \t<key>CFBundlePackageType</key>\n\t<string>APPL</string>\n\
         \t<key>CFBundleShortVersionString</key>\n\t<string>{version}</string>\n\
         \t<key>CFBundleVersion</key>\n\t<string>{version}</string>\n\
         \t<key>LSMinimumSystemVersion</key>\n\t<string>11.0</string>\n\
         \t<key>NSHighResolutionCapable</key>\n\t<true/>\n\
         \t<key>CFBundleDocumentTypes</key>\n\t<array>\n{doc_types}\t</array>\n\
         </dict>\n</plist>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_format_appears_in_the_plist() {
        let plist = info_plist("1.2.3");
        for format in FORMATS {
            assert!(
                plist.contains(&format!("<string>{}</string>", format.extension)),
                "{} missing from plist",
                format.extension
            );
        }
        assert!(plist.contains("<string>1.2.3</string>"));
    }

    #[test]
    fn every_decodable_extension_can_be_registered() {
        let registered = extensions();
        for ext in crate::decode::supported_extensions() {
            assert!(
                registered.contains(&ext),
                "{ext} decodes but is missing from the association table"
            );
        }
    }

    #[test]
    fn desktop_entry_mimes_match_the_format_table() {
        let desktop = include_str!("../resource/io.github.knst0.reveal.desktop");
        for mime in mime_list() {
            assert!(desktop.contains(mime), "{mime} missing from reveal.desktop");
        }
    }
}
