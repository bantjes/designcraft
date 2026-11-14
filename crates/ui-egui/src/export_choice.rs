//! File › Export… (Cmd-E): one window to choose what to export the document as. Each format
//! goes on to its own export (its options, then where to save, named after the document).

use serde_json::{Value, json};

use crate::DesignApp;
use crate::theme::Tokens;

/// The formats offered: (key, label, the command that exports it).
pub const FORMATS: &[(&str, &str, &str)] = &[
    ("pdf", "PDF (Print)", "app.exportPdf"),
    ("interactivePdf", "PDF (Interactive)", "app.exportInteractivePdf"),
    ("image", "TIFF or JPEG", "app.exportImage"),
    ("png", "PNG", "app.exportPng"),
    ("epub", "EPUB (Reflowable)", "app.exportEpub"),
    ("fixedEpub", "EPUB (Fixed Layout)", "app.exportFixedEpub"),
    ("html", "HTML", "app.exportHtml"),
    ("idml", "IDML (InDesign Markup)", "app.exportIdml"),
    ("pptx", "PowerPoint", "app.exportPptx"),
    ("xml", "XML", "app.exportXml"),
];

/// The command for `key`, the first format's when `key` is unknown.
fn command_for(key: &str) -> &'static str {
    FORMATS.iter().find(|(k, ..)| *k == key).or(FORMATS.first()).map_or("app.exportPdf", |(_, _, c)| c)
}

/// `app.exportDialog {show?, format?}`: opens (or closes) the window; with `format`, exports in
/// that format at once. `None` for other ids.
pub fn command(app: &mut DesignApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    if id != "app.exportDialog" {
        return None;
    }
    if app.session.active().is_none() {
        return Some(Err("no document".into()));
    }
    if let Some(key) = p.get("format").and_then(Value::as_str) {
        if !FORMATS.iter().any(|(k, ..)| *k == key) {
            return Some(Err(format!("app.exportDialog: unknown format {key:?}")));
        }
        app.ui.export_choice = key.to_string();
        app.ui.export_choice_open = false;
        return Some(app.run(command_for(key), json!({})));
    }
    let open = p.get("show").and_then(Value::as_bool).unwrap_or(true);
    app.ui.export_choice_open = open;
    Some(Ok(json!({ "visible": open })))
}

/// Draws the window when it is open.
pub fn window(app: &mut DesignApp, ctx: &egui::Context) {
    if !app.ui.export_choice_open {
        return;
    }
    if app.session.active().is_none() {
        app.ui.export_choice_open = false;
        return;
    }
    let t = Tokens::get(ctx);
    let lang = app.ui.language.clone();
    let tr = |s: &'static str| crate::i18n::tr(&lang, s).to_owned();
    let mut choice = app.ui.export_choice.clone();
    if !FORMATS.iter().any(|(k, ..)| *k == choice) {
        choice = FORMATS.first().map(|(k, ..)| k.to_string()).unwrap_or_default();
    }
    let mut open = true;
    let mut export = false;
    let mut cancel = false;
    egui::Window::new(tr("Export"))
        .id(egui::Id::unique("export-choice-window"))
        .open(&mut open)
        .default_pos(ctx.content_rect().center() - egui::vec2(140.0, 180.0))
        .default_width(280.0)
        .resizable(false)
        .collapsible(false)
        .frame(egui::Frame::window(&ctx.global_style()).fill(t.panel))
        .show(ctx, |ui| {
            ui.label(tr("Export the document as:"));
            ui.add_space(4.0);
            for (key, label, _) in FORMATS {
                let r = ui.radio_value(&mut choice, key.to_string(), tr(label));
                if r.double_clicked() {
                    export = true;
                }
            }
            ui.add_space(6.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("Export…")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        export = true;
                    }
                    if ui.button(tr("Cancel")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        cancel = true;
                    }
                });
            });
        });
    app.ui.export_choice = choice.clone();
    if export {
        app.ui.export_choice_open = false;
        if let Err(e) = app.run(command_for(&choice), json!({})) {
            app.status(e);
        }
    } else if !open || cancel {
        app.ui.export_choice_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> DesignApp {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        app
    }

    #[test]
    fn cmd_e_opens_the_export_window() {
        let mut app = app();
        assert_eq!(crate::menus::default_shortcut("app.exportDialog"), Some("Cmd+E"));
        app.run("app.exportDialog", json!({})).unwrap();
        assert!(app.ui.export_choice_open);
    }

    #[test]
    fn every_format_names_a_command_and_the_choice_is_remembered() {
        for (_, _, cmd) in FORMATS {
            assert!(crate::menus::UI_COMMANDS.iter().any(|(id, ..)| id == cmd), "{cmd} is not a UI command");
        }
        let mut app = app();
        app.run("app.exportDialog", json!({"format": "image"})).unwrap();
        assert_eq!(app.ui.export_choice, "image");
        assert!(app.ui.export_image_open, "TIFF or JPEG goes on to its own window");
        assert!(app.run("app.exportDialog", json!({"format": "nope"})).is_err());
    }
}
