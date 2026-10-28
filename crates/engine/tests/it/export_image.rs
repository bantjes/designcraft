//! File › Export Page as TIFF or JPEG: size, resolution, profile, compression and rich black.
use designcraft_engine::Session;
use serde_json::json;

fn doc() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    let id = s.execute("frame.create", &json!({"rect": [0, 0, 100, 100], "content": "unassigned"})).unwrap()["id"].clone();
    s.execute("object.fill", &json!({"ids": [id], "swatch": "[Black]"})).unwrap();
    s
}

fn bytes(r: &serde_json::Value) -> Vec<u8> {
    designcraft_engine::cmd::base64_decode(r["base64"].as_str().unwrap())
}

#[test]
fn a_page_exports_as_jpeg_at_the_resolution_and_size_asked_for() {
    let mut s = doc();
    let page_w = s.execute("file.exportImage", &json!({"ppi": 72})).unwrap()["width"].as_u64().unwrap();
    let r = s.execute("file.exportImage", &json!({"ppi": 144, "scale": 50, "quality": 40})).unwrap();
    assert_eq!(r["width"].as_u64().unwrap(), page_w);
    assert_eq!(r["ppi"], 144);
    let b = bytes(&r);
    assert!(b.starts_with(&[0xFF, 0xD8, 0xFF]));
    let r = s.execute("file.exportImage", &json!({"width": 500})).unwrap();
    assert_eq!(r["width"], 500);
}

#[test]
fn tiff_in_adobe_rgb_carries_that_profile() {
    let mut s = doc();
    for compression in ["none", "lzw", "zip"] {
        let r = s.execute("file.exportImage", &json!({"format": "tiff", "profile": "adobeRGB", "compression": compression, "ppi": 72})).unwrap();
        let b = bytes(&r);
        assert!(b.starts_with(b"II*\0") || b.starts_with(b"MM\0*"), "{compression}");
        assert!(b.windows(32).any(|w| w == b"Compatible with Adobe RGB (1998)"), "{compression}");
    }
}

#[test]
fn rich_black_makes_100_percent_black_deeper() {
    let mut s = doc();
    let darkest = |s: &mut Session, rich: bool| {
        let r = s.execute("file.exportImage", &json!({"format": "tiff", "compression": "none", "ppi": 36, "richBlack": rich})).unwrap();
        let px = image::load_from_memory_with_format(&bytes(&r), image::ImageFormat::Tiff).unwrap().to_rgb8().into_raw();
        px.chunks(3).take(4).map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])).min().unwrap()
    };
    let (plain, rich) = (darkest(&mut s, false), darkest(&mut s, true));
    assert!(rich < plain, "rich black {rich} should be darker than plain {plain}");
    assert_eq!(rich, 0);
}

#[test]
fn bad_settings_are_errors() {
    let mut s = doc();
    for p in [
        json!({"format": "gif"}),
        json!({"profile": "prophoto"}),
        json!({"ppi": 0}),
        json!({"ppi": 1e9}),
        json!({"scale": -5}),
        json!({"quality": "high"}),
        json!({"compression": "jpeg", "format": "tiff"}),
        json!({"page": 99}),
        json!({"width": 0}),
        json!({"richBlack": "yes"}),
    ] {
        assert!(s.execute("file.exportImage", &p).is_err(), "{p}");
    }
    // Too many pixels: an error, not a crash.
    assert!(s.execute("file.exportImage", &json!({"ppi": 2400, "scale": 1000})).is_err());
}
