//! File › Export Page as JPEG or TIFF…: a window with the page, format, colour profile,
//! resolution, size, JPEG quality or TIFF compression, rich black and bleed; Export asks where
//! to save and writes what `file.exportImage` makes. The choices are kept for next time.

use egui::RichText;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::DesignApp;
use crate::theme::Tokens;

/// The window's own commands.
pub const IDS: [&str; 1] = ["app.exportImage"];

/// How the size is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SizeBy {
    Percent,
    Pixels,
    Inches,
    Centimetres,
}

/// The last choices made in the window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExportImageSettings {
    pub tiff: bool,
    pub adobe_rgb: bool,
    pub ppi: f64,
    pub size_by: SizeBy,
    /// The size in `size_by`'s unit (a percentage, or the width).
    pub size: f64,
    pub quality: u8,
    /// none, lzw or zip.
    pub compression: String,
    pub rich_black: bool,
    pub bleed: bool,
}

impl Default for ExportImageSettings {
    fn default() -> Self {
        Self {
            tiff: false,
            adobe_rgb: false,
            ppi: 300.0,
            size_by: SizeBy::Percent,
            size: 100.0,
            quality: 90,
            compression: "lzw".into(),
            rich_black: true,
            bleed: false,
        }
    }
}

impl ExportImageSettings {
    /// The `file.exportImage` parameters for `page`, whose width is `page_w` points.
    pub fn params(&self, page: usize) -> Value {
        let mut p = json!({
            "page": page,
            "format": if self.tiff { "tiff" } else { "jpeg" },
            "profile": if self.adobe_rgb { "adobeRGB" } else { "sRGB" },
            "ppi": self.ppi,
            "quality": self.quality,
            "compression": self.compression,
            "richBlack": self.rich_black,
            "bleed": self.bleed,
        });
        let inches = match self.size_by {
            SizeBy::Percent => None,
            SizeBy::Pixels => {
                p["width"] = json!(self.size.round());
                None
            }
            SizeBy::Inches => Some(self.size),
            SizeBy::Centimetres => Some(self.size / 2.54),
        };
        match inches {
            Some(w) => p["width"] = json!((w * self.ppi).round()),
            None if self.size_by == SizeBy::Percent => p["scale"] = json!(self.size),
            None => {}
        }
        p
    }

    /// The pixel size the file will have for a page of `w`×`h` points.
    pub fn pixels(&self, w: f64, h: f64) -> (f64, f64) {
        let width = match self.size_by {
            SizeBy::Percent => w / 72.0 * self.ppi * self.size / 100.0,
            SizeBy::Pixels => self.size,
            SizeBy::Inches => self.size * self.ppi,
            SizeBy::Centimetres => self.size / 2.54 * self.ppi,
        }
        .round()
        .max(1.0);
        (width, (width * h / w.max(1.0)).round().max(1.0))
    }
}

/// `app.exportImage {show?}` opens or closes the window; `None` for other ids.
pub fn command(app: &mut DesignApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    if id != "app.exportImage" {
        return None;
    }
    if app.session.active().is_none() {
        return Some(Err("no document".into()));
    }
    let open = p.get("show").and_then(Value::as_bool).unwrap_or(true);
    app.ui.export_image_open = open;
    if open {
        app.ui.export_image_page = crate::canvas::current_page(app).unwrap_or(0);
    }
    Some(Ok(json!({ "visible": open })))
}

/// The page's size in points (with its bleed when `bleed`).
fn page_size(app: &DesignApp, page: usize, bleed: bool) -> Option<(f64, f64)> {
    let d = &app.session.active()?.doc;
    let (si, pi) = d.page_loc(page)?;
    let b = d.spreads.get(si)?.pages.get(pi)?.bounds();
    let extra = if bleed { d.settings.bleed } else { [0.0; 4] };
    Some((b.width() + extra[2] + extra[3], b.height() + extra[0] + extra[1]))
}

/// Draws the window when it is open.
pub fn window(app: &mut DesignApp, ctx: &egui::Context) {
    if !app.ui.export_image_open {
        return;
    }
    let Some(count) = app.session.active().map(|d| d.doc.page_count()) else {
        app.ui.export_image_open = false;
        return;
    };
    let t = Tokens::get(ctx);
    let lang = app.ui.language.clone();
    let tr = |s: &'static str| crate::i18n::tr(&lang, s).to_owned();
    let mut s = app.ui.export_image.clone();
    let mut page = app.ui.export_image_page.min(count.saturating_sub(1));
    let size = page_size(app, page, s.bleed);
    let mut open = true;
    let mut export = false;
    let mut cancel = false;
    egui::Window::new(tr("Export Page as JPEG or TIFF"))
        .id(egui::Id::unique("export-image-window"))
        .open(&mut open)
        .default_pos(ctx.content_rect().center() - egui::vec2(180.0, 200.0))
        .default_width(360.0)
        .resizable(false)
        .collapsible(false)
        .frame(egui::Frame::window(&ctx.global_style()).fill(t.panel))
        .show(ctx, |ui| {
            egui::Grid::new("export-image-grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(tr("Page:"));
                let mut shown = page + 1;
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut shown).range(1..=count.max(1)).speed(0.1));
                    ui.label(RichText::new(format!("/ {count}")).color(t.text_dim));
                });
                page = shown.saturating_sub(1);
                ui.end_row();

                ui.label(tr("Format:"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.tiff, false, "JPEG");
                    ui.radio_value(&mut s.tiff, true, "TIFF");
                });
                ui.end_row();

                ui.label(tr("Colour Profile:"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.adobe_rgb, false, "sRGB");
                    ui.radio_value(&mut s.adobe_rgb, true, "Adobe RGB (1998)");
                });
                ui.end_row();

                ui.label(tr("Resolution:"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut s.ppi).range(1.0..=2400.0).speed(1.0).max_decimals(0).suffix(" ppi"));
                    for preset in [72.0, 150.0, 300.0] {
                        if ui.selectable_label(s.ppi == preset, format!("{preset}")).clicked() {
                            s.ppi = preset;
                        }
                    }
                });
                ui.end_row();

                ui.label(tr("Size:"));
                ui.horizontal(|ui| {
                    let before = s.size_by;
                    let (range, suffix, decimals) = match s.size_by {
                        SizeBy::Percent => (1.0..=1000.0, " %", 0),
                        SizeBy::Pixels => (1.0..=60000.0, " px", 0),
                        SizeBy::Inches => (0.1..=500.0, " in", 2),
                        SizeBy::Centimetres => (0.1..=1270.0, " cm", 1),
                    };
                    ui.add(egui::DragValue::new(&mut s.size).range(range).speed(0.5).max_decimals(decimals).suffix(suffix));
                    craft_ui::ComboBox::from_id_salt("export-image-size-by").selected_text(tr(size_label(s.size_by))).show_ui(ui, |ui| {
                        for by in [SizeBy::Percent, SizeBy::Pixels, SizeBy::Inches, SizeBy::Centimetres] {
                            ui.selectable_value(&mut s.size_by, by, tr(size_label(by)));
                        }
                    });
                    // A new unit starts from the size the file has now.
                    if s.size_by != before
                        && let Some((w, h)) = size
                    {
                        let px = ExportImageSettings { size_by: before, ..s.clone() }.pixels(w, h).0;
                        s.size = match s.size_by {
                            SizeBy::Percent => (px / (w / 72.0 * s.ppi) * 100.0).round(),
                            SizeBy::Pixels => px,
                            SizeBy::Inches => (px / s.ppi * 100.0).round() / 100.0,
                            SizeBy::Centimetres => (px / s.ppi * 2.54 * 10.0).round() / 10.0,
                        };
                    }
                });
                ui.end_row();

                if s.tiff {
                    ui.label(tr("Compression:"));
                    craft_ui::ComboBox::from_id_salt("export-image-compression").selected_text(tr(compression_label(&s.compression))).show_ui(
                        ui,
                        |ui| {
                            for c in ["none", "lzw", "zip"] {
                                ui.selectable_value(&mut s.compression, c.to_string(), tr(compression_label(c)));
                            }
                        },
                    );
                } else {
                    ui.label(tr("Quality:"));
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut s.quality, 1..=100));
                        ui.label(RichText::new(tr(quality_label(s.quality))).color(t.text_dim));
                    });
                }
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.checkbox(&mut s.rich_black, tr("Make black rich black"));
            ui.checkbox(&mut s.bleed, tr("Include the bleed"));
            ui.add_space(4.0);
            if let Some((w, h)) = size {
                let (pw, ph) = s.pixels(w, h);
                let (iw, ih) = (pw / s.ppi, ph / s.ppi);
                let mb = pw * ph * 3.0 / 1_000_000.0;
                let too_big =
                    pw > f64::from(designcraft_render::MAX_SIDE) || ph > f64::from(designcraft_render::MAX_SIDE) || pw * ph > (1u64 << 29) as f64;
                let line = format!(
                    "{pw:.0} × {ph:.0} px · {iw:.2} × {ih:.2} in · {:.1} × {:.1} cm · {mb:.0} MB {}",
                    iw * 2.54,
                    ih * 2.54,
                    tr("uncompressed")
                );
                ui.label(RichText::new(line).size(11.0).color(if too_big { t.caption_close } else { t.text_dim }));
                if too_big {
                    ui.label(RichText::new(tr("Too large: lower the resolution or the size.")).size(11.0).color(t.caption_close));
                }
            }
            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("Export…")).clicked() {
                        export = true;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        });
    app.ui.export_image = s;
    app.ui.export_image_page = page;
    if export {
        match export_now(app) {
            Ok(Some(path)) => {
                app.ui.export_image_open = false;
                app.status(format!("Exported {path}"));
            }
            // Cancelled in the save dialog: the window stays.
            Ok(None) => {}
            Err(e) => app.status(e),
        }
    }
    if !open || cancel {
        app.ui.export_image_open = false;
    }
}

/// Ask where to save and write the file; `None` when the save dialog was cancelled.
fn export_now(app: &mut DesignApp) -> Result<Option<String>, String> {
    let page = app.ui.export_image_page;
    let p = app.ui.export_image.params(page);
    let title = app.session.active().map(|d| d.doc.title.clone()).unwrap_or_default();
    let ext = if app.ui.export_image.tiff { "tif" } else { "jpg" };
    let Some(path) = app.pick_save(&format!("{title}-p{}.{ext}", page + 1)) else { return Ok(None) };
    let r = app.run("file.exportImage", p)?;
    let bytes = designcraft_engine::cmd::base64_decode(r["base64"].as_str().unwrap_or_default());
    let write = app.services.write.as_mut().ok_or("no writer")?;
    write(&path, &bytes)?;
    Ok(Some(path))
}

fn size_label(by: SizeBy) -> &'static str {
    match by {
        SizeBy::Percent => "Percent of page",
        SizeBy::Pixels => "Width in pixels",
        SizeBy::Inches => "Width in inches",
        SizeBy::Centimetres => "Width in cm",
    }
}

fn compression_label(c: &str) -> &'static str {
    match c {
        "none" => "None",
        "zip" => "ZIP (lossless)",
        _ => "LZW (lossless)",
    }
}

fn quality_label(q: u8) -> &'static str {
    match q {
        0..=39 => "Low",
        40..=69 => "Medium",
        70..=89 => "High",
        _ => "Maximum",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> DesignApp {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({})).unwrap();
        app
    }

    #[test]
    fn the_window_opens_on_the_current_page_and_needs_a_document() {
        let mut none = DesignApp::new(designcraft_engine::Session::new(), Default::default());
        assert!(none.run("app.exportImage", json!({})).is_err());
        let mut app = app();
        app.run("app.exportImage", json!({})).unwrap();
        assert!(app.ui.export_image_open);
        app.run("app.exportImage", json!({"show": false})).unwrap();
        assert!(!app.ui.export_image_open);
    }

    #[test]
    fn every_size_unit_gives_the_engine_the_pixels_the_window_shows() {
        let mut app = app();
        let (w, h) = page_size(&app, 0, false).unwrap();
        for (by, size) in [(SizeBy::Percent, 50.0), (SizeBy::Pixels, 1200.0), (SizeBy::Inches, 4.0), (SizeBy::Centimetres, 10.0)] {
            for tiff in [false, true] {
                let s = ExportImageSettings { size_by: by, size, ppi: 150.0, tiff, adobe_rgb: tiff, ..Default::default() };
                let r = app.run("file.exportImage", s.params(0)).unwrap();
                let (pw, ph) = s.pixels(w, h);
                assert_eq!(r["width"].as_f64().unwrap(), pw, "{by:?}");
                assert!((r["height"].as_f64().unwrap() - ph).abs() <= 1.0, "{by:?}");
            }
        }
    }

    #[test]
    fn exporting_writes_the_file_the_save_dialog_names() {
        let written = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(String, Vec<u8>)>::new()));
        let mut app = app();
        let sink = written.clone();
        app.services.write = Some(Box::new(move |path: &str, bytes: &[u8]| {
            sink.borrow_mut().push((path.to_string(), bytes.to_vec()));
            Ok(())
        }));
        app.services.pick_save = Some(Box::new(|name: &str| Some(format!("/out/{name}"))));
        app.ui.export_image = ExportImageSettings { tiff: true, ppi: 72.0, ..Default::default() };
        let path = export_now(&mut app).unwrap().unwrap();
        assert!(path.ends_with("-p1.tif"), "{path}");
        let w = written.borrow();
        assert_eq!(w.len(), 1);
        assert!(w[0].1.starts_with(b"II*\0") || w[0].1.starts_with(b"MM\0*"));
    }
}
