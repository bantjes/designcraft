//! A rendered page as a JPEG or TIFF file: its pixels in sRGB or Adobe RGB (1998), the profile
//! embedded, the resolution recorded, and the compression chosen.
//!
//! Adobe RGB (1998) values come from Adobe's published specification of the colour space
//! (primaries, D65 white, gamma 563/256); the profile embedded for it is built here, so no
//! profile file is copied. sRGB embeds `assets/icc/sRGB-v2-magic.icc` (CC0).

use std::io::Cursor;

use image::ImageEncoder;

/// The embedded sRGB profile (CC0, see ASSETS.md).
static SRGB_ICC: &[u8] = include_bytes!("../../../assets/icc/sRGB-v2-magic.icc");

/// The colour space the file's pixels are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Srgb,
    AdobeRgb,
}

impl Profile {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace([' ', '-', '_'], "").as_str() {
            "srgb" => Some(Self::Srgb),
            "adobergb" | "adobergb1998" | "adobergb(1998)" => Some(Self::AdobeRgb),
            _ => None,
        }
    }
}

/// TIFF compression (all lossless).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TiffCompression {
    None,
    Lzw,
    Zip,
}

impl TiffCompression {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "lzw" => Some(Self::Lzw),
            "zip" | "deflate" => Some(Self::Zip),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Quality 1 (smallest file) to 100 (best).
    Jpeg {
        quality: u8,
    },
    Tiff {
        compression: TiffCompression,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    pub format: Format,
    pub profile: Profile,
    /// Pixels per inch, recorded in the file.
    pub ppi: u16,
}

/// Encode `rgba` (straight, not premultiplied, sRGB, `width`×`height`) as the file `opts` asks
/// for. Transparent areas become white, as on paper.
pub fn encode_page(rgba: &[u8], width: u32, height: u32, opts: &ExportOptions) -> Result<Vec<u8>, String> {
    let n = (width as usize).checked_mul(height as usize).ok_or("the image is too large")?;
    if width == 0 || height == 0 || rgba.len() < n.saturating_mul(4) {
        return Err(format!("{width}×{height} image: not enough pixels"));
    }
    let mut rgb = flatten_on_white(rgba.get(..n * 4).unwrap_or_default());
    let icc: Vec<u8> = match opts.profile {
        Profile::Srgb => SRGB_ICC.to_vec(),
        Profile::AdobeRgb => {
            srgb_to_adobe_rgb(&mut rgb);
            adobe_rgb_icc()
        }
    };
    match opts.format {
        Format::Jpeg { quality } => jpeg(&rgb, width, height, quality.clamp(1, 100), opts.ppi, icc),
        Format::Tiff { compression } => tiff(&rgb, width, height, compression, opts.ppi, &icc),
    }
}

fn flatten_on_white(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let a = u32::from(p[3]);
            let mix = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255).min(255) as u8;
            [mix(p[0]), mix(p[1]), mix(p[2])]
        })
        .collect()
}

/// Linear sRGB → linear Adobe RGB (1998); both have the D65 white, so no adaptation.
const SRGB_TO_ADOBE: [[f32; 3]; 3] = [[0.715_126, 0.284_874, 0.0], [0.0, 1.0, 0.0], [0.0, 0.041_162, 0.958_838]];
/// Adobe RGB (1998)'s gamma, 2 51/256.
const ADOBE_GAMMA: f32 = 563.0 / 256.0;
/// Steps of the linear → Adobe RGB encoding table (fine enough for 8-bit output in the shadows).
const ENCODE_STEPS: usize = 4096;

fn srgb_to_adobe_rgb(rgb: &mut [u8]) {
    let decode: Vec<f32> = (0..256u16)
        .map(|v| {
            let c = f32::from(v) / 255.0;
            if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        })
        .collect();
    let encode: Vec<u8> =
        (0..=ENCODE_STEPS).map(|i| ((i as f32 / ENCODE_STEPS as f32).powf(1.0 / ADOBE_GAMMA) * 255.0).round().clamp(0.0, 255.0) as u8).collect();
    let lin = |v: u8| decode.get(usize::from(v)).copied().unwrap_or(0.0);
    let enc = |v: f32| encode.get((v.clamp(0.0, 1.0) * ENCODE_STEPS as f32).round() as usize).copied().unwrap_or(255);
    for px in rgb.as_chunks_mut::<3>().0 {
        let l = [lin(px[0]), lin(px[1]), lin(px[2])];
        for (out, row) in px.iter_mut().zip(SRGB_TO_ADOBE) {
            *out = enc(row[0] * l[0] + row[1] * l[1] + row[2] * l[2]);
        }
    }
}

fn jpeg(rgb: &[u8], width: u32, height: u32, quality: u8, ppi: u16, icc: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    enc.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(ppi.max(1)));
    enc.set_icc_profile(icc).map_err(|e| format!("JPEG colour profile: {e}"))?;
    enc.write_image(rgb, width, height, image::ExtendedColorType::Rgb8).map_err(|e| format!("JPEG: {e}"))?;
    Ok(out)
}

/// A TIFF value of type UNDEFINED (the ICC profile tag's type).
struct Undefined<'a>(&'a [u8]);

impl tiff::encoder::TiffValue for Undefined<'_> {
    const BYTE_LEN: u8 = 1;
    const FIELD_TYPE: tiff::tags::Type = tiff::tags::Type::UNDEFINED;
    fn count(&self) -> usize {
        self.0.len()
    }
    fn data(&self) -> std::borrow::Cow<'_, [u8]> {
        std::borrow::Cow::Borrowed(self.0)
    }
}

fn tiff(rgb: &[u8], width: u32, height: u32, compression: TiffCompression, ppi: u16, icc: &[u8]) -> Result<Vec<u8>, String> {
    use tiff::encoder::{Compression, DeflateLevel, Predictor, TiffEncoder, colortype::RGB8};
    let err = |e: tiff::TiffError| format!("TIFF: {e}");
    let mut out = Cursor::new(Vec::new());
    {
        let (method, predictor) = match compression {
            TiffCompression::None => (Compression::Uncompressed, Predictor::None),
            TiffCompression::Lzw => (Compression::Lzw, Predictor::Horizontal),
            TiffCompression::Zip => (Compression::Deflate(DeflateLevel::Balanced), Predictor::Horizontal),
        };
        let mut enc = TiffEncoder::new(&mut out).map_err(err)?.with_compression(method).with_predictor(predictor);
        let mut image = enc.new_image::<RGB8>(width, height).map_err(err)?;
        image.resolution(tiff::tags::ResolutionUnit::Inch, tiff::encoder::Rational { n: u32::from(ppi.max(1)), d: 1 });
        image.encoder().write_tag(tiff::tags::Tag::IccProfile, Undefined(icc)).map_err(err)?;
        image.write_data(rgb).map_err(err)?;
    }
    Ok(out.into_inner())
}

/// An ICC v2 display profile for Adobe RGB (1998): its D50-adapted colorants and white from
/// the published specification, one gamma curve of 563/256 for each channel.
pub fn adobe_rgb_icc() -> Vec<u8> {
    fn s15(v: f64) -> [u8; 4] {
        ((v * 65536.0).round() as i32).to_be_bytes()
    }
    fn xyz(v: [f64; 3]) -> Vec<u8> {
        let mut t = b"XYZ \0\0\0\0".to_vec();
        for c in v {
            t.extend(s15(c));
        }
        t
    }
    let name = "Compatible with Adobe RGB (1998)";
    let mut desc = b"desc\0\0\0\0".to_vec();
    desc.extend((name.len() as u32 + 1).to_be_bytes());
    desc.extend(name.as_bytes());
    desc.push(0);
    // No Unicode or ScriptCode descriptions.
    desc.extend([0u8; 4 + 4 + 2 + 1 + 67]);
    let mut cprt = b"text\0\0\0\0".to_vec();
    cprt.extend(b"No copyright, use freely (CC0). Built by Marian DesignCraft.\0");
    let mut curve = b"curv\0\0\0\0".to_vec();
    curve.extend(1u32.to_be_bytes());
    curve.extend(0x0233u16.to_be_bytes());
    let tags: [(&[u8; 4], Vec<u8>); 9] = [
        (b"desc", desc),
        (b"cprt", cprt),
        (b"wtpt", xyz([0.950_45, 1.0, 1.089_05])),
        (b"rXYZ", xyz([0.609_74, 0.311_11, 0.019_47])),
        (b"gXYZ", xyz([0.205_28, 0.625_67, 0.060_87])),
        (b"bXYZ", xyz([0.149_19, 0.063_22, 0.744_57])),
        (b"rTRC", curve.clone()),
        (b"gTRC", curve.clone()),
        (b"bTRC", curve),
    ];
    let mut table = Vec::new();
    let mut data = Vec::new();
    let start = 128 + 4 + 12 * tags.len();
    for (sig, body) in &tags {
        while data.len() % 4 != 0 {
            data.push(0);
        }
        table.extend(*sig);
        table.extend(((start + data.len()) as u32).to_be_bytes());
        table.extend((body.len() as u32).to_be_bytes());
        data.extend(body);
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let size = start + data.len();
    let mut out = Vec::with_capacity(size);
    out.extend((size as u32).to_be_bytes());
    out.extend([0u8; 4]); // preferred CMM
    out.extend(0x0210_0000u32.to_be_bytes()); // version 2.1
    out.extend(b"mntrRGB XYZ ");
    for v in [2026u16, 10, 28, 0, 0, 0] {
        out.extend(v.to_be_bytes());
    }
    out.extend(b"acsp");
    out.extend([0u8; 4 + 4 + 4 + 4 + 8 + 4]); // platform, flags, maker, model, attributes, intent
    for c in [0.964_2, 1.0, 0.824_9] {
        out.extend(s15(c)); // PCS illuminant (D50)
    }
    out.extend([0u8; 4 + 44]); // creator, reserved
    out.extend((tags.len() as u32).to_be_bytes());
    out.extend(table);
    out.extend(data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Vec<u8> {
        // Black, red, white and transparent pixels.
        [[0, 0, 0, 255], [255, 0, 0, 255], [255, 255, 255, 255], [0, 0, 0, 0]].concat()
    }

    #[test]
    fn jpeg_carries_its_profile_and_resolution() {
        for profile in [Profile::Srgb, Profile::AdobeRgb] {
            let opts = ExportOptions { format: Format::Jpeg { quality: 90 }, profile, ppi: 300 };
            let bytes = encode_page(&page(), 2, 2, &opts).unwrap();
            assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
            let icc = if profile == Profile::Srgb { SRGB_ICC.to_vec() } else { adobe_rgb_icc() };
            assert!(bytes.windows(12).any(|w| w == b"ICC_PROFILE\0"));
            assert!(bytes.windows(icc.len().min(64)).any(|w| w == &icc[..icc.len().min(64)]));
            let mut dec = image::codecs::jpeg::JpegDecoder::new(Cursor::new(&bytes)).unwrap();
            use image::ImageDecoder;
            assert_eq!(dec.icc_profile().unwrap().unwrap(), icc);
            assert_eq!(dec.dimensions(), (2, 2));
        }
    }

    #[test]
    fn tiff_reads_back_with_its_pixels_profile_and_resolution() {
        for compression in [TiffCompression::None, TiffCompression::Lzw, TiffCompression::Zip] {
            let opts = ExportOptions { format: Format::Tiff { compression }, profile: Profile::Srgb, ppi: 240 };
            let bytes = encode_page(&page(), 2, 2, &opts).unwrap();
            let mut dec = tiff::decoder::Decoder::new(Cursor::new(&bytes)).unwrap();
            assert_eq!(dec.dimensions().unwrap(), (2, 2));
            let tiff::decoder::DecodingResult::U8(px) = dec.read_image().unwrap() else { panic!("8-bit RGB") };
            // The transparent pixel is white, as on paper.
            assert_eq!(px, [0, 0, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]);
            assert_eq!(dec.get_tag_u8_vec(tiff::tags::Tag::IccProfile).unwrap(), SRGB_ICC);
            let x = dec.get_tag(tiff::tags::Tag::XResolution).unwrap();
            assert_eq!(format!("{x:?}"), format!("{:?}", tiff::decoder::ifd::Value::Rational(240, 1)));
        }
    }

    #[test]
    fn adobe_rgb_keeps_black_white_and_grey_and_pulls_srgb_red_in() {
        let mut px = vec![0, 0, 0, 255, 255, 255, 128, 128, 128, 255, 0, 0, 0, 255, 0];
        srgb_to_adobe_rgb(&mut px);
        assert_eq!(&px[..6], &[0, 0, 0, 255, 255, 255]);
        // Neutral greys stay neutral.
        assert!(px[6] == px[7] && px[7] == px[8], "{px:?}");
        // sRGB's primaries are inside Adobe RGB's wider gamut: red shares Adobe RGB's red primary
        // (less of it), green takes some red and blue.
        assert_eq!(&px[9..12], &[219, 0, 0], "{px:?}");
        assert!(px[12] > 120 && px[12] < 160 && px[13] == 255 && px[14] > 40 && px[14] < 80, "{px:?}");
    }

    #[test]
    fn the_adobe_rgb_profile_is_well_formed() {
        let icc = adobe_rgb_icc();
        assert_eq!(u32::from_be_bytes(icc[..4].try_into().unwrap()) as usize, icc.len());
        assert_eq!(&icc[36..40], b"acsp");
        assert_eq!(&icc[12..24], b"mntrRGB XYZ ");
        let count = u32::from_be_bytes(icc[128..132].try_into().unwrap()) as usize;
        for i in 0..count {
            let e = &icc[132 + 12 * i..144 + 12 * i];
            let (off, len) = (u32::from_be_bytes(e[4..8].try_into().unwrap()) as usize, u32::from_be_bytes(e[8..12].try_into().unwrap()) as usize);
            assert!(off % 4 == 0 && off + len <= icc.len(), "tag {i}");
        }
        assert!(Profile::parse("Adobe RGB (1998)") == Some(Profile::AdobeRgb) && Profile::parse("sRGB") == Some(Profile::Srgb));
    }

    #[test]
    fn too_few_pixels_is_an_error_not_a_crash() {
        let opts = ExportOptions { format: Format::Jpeg { quality: 50 }, profile: Profile::Srgb, ppi: 72 };
        assert!(encode_page(&[0; 8], 2, 2, &opts).is_err());
        assert!(encode_page(&[], 0, 0, &opts).is_err());
    }
}
