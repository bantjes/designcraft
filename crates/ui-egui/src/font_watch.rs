//! New fonts appear without a restart: every few seconds (while the app is drawing) the font
//! folders' modified times are checked, and when one changed, because a font was installed or
//! removed, the fonts are read again. No timer of its own: coming back to the app from Font Book
//! draws a frame, and that frame looks.

use serde_json::json;

use crate::DesignApp;

/// Seconds between looks at the font folders.
const EVERY: f64 = 3.0;

#[derive(Clone, Copy, Default)]
struct Watch {
    next: f64,
    signature: Option<u64>,
}

/// Looks at the font folders when it is time to.
pub(crate) fn tick(app: &mut DesignApp, ctx: &egui::Context) {
    let now = ctx.input(|i| i.time);
    let id = egui::Id::unique("font-folder-watch");
    let mut w: Watch = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    if now < w.next {
        return;
    }
    w.next = now + EVERY;
    let signature = designcraft_fonts::font_dirs_signature(&designcraft_fonts::system_font_dirs());
    step(app, &mut w.signature, signature);
    ctx.data_mut(|d| d.insert_temp(id, w));
}

/// Reloads the fonts when `signature` differs from the last one seen (never on the first look).
/// True when it reloaded.
fn step(app: &mut DesignApp, last: &mut Option<u64>, signature: u64) -> bool {
    let changed = last.is_some_and(|l| l != signature);
    *last = Some(signature);
    if changed {
        match app.run("font.reload", json!({})) {
            Ok(_) => {
                let msg = crate::i18n::tr(&app.ui.language, "Fonts updated").to_owned();
                app.status(msg);
            }
            Err(e) => app.status(e),
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_in_the_font_folders_reloads_the_fonts() {
        let mut app = DesignApp::new(designcraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({})).unwrap();
        let mut last = None;
        assert!(!step(&mut app, &mut last, 1), "the first look only remembers");
        assert!(!step(&mut app, &mut last, 1), "nothing changed");
        let revision = app.session.doc().unwrap().revision;
        assert!(step(&mut app, &mut last, 2), "a font was added");
        assert_ne!(app.session.doc().unwrap().revision, revision, "the document is drawn with the new fonts");
        let dir = std::env::temp_dir().join(format!("dc-font-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let before = designcraft_fonts::font_dirs_signature(std::slice::from_ref(&dir));
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::create_dir_all(dir.join("New Family")).unwrap();
        let after = designcraft_fonts::font_dirs_signature(std::slice::from_ref(&dir));
        std::fs::remove_dir_all(&dir).ok();
        assert_ne!(before, after, "a new folder in a font folder changes the signature");
    }
}
