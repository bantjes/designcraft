//! New fonts appear without a restart: every few seconds (while the app is drawing) the font
//! folders' modified times are checked, and when one changed, because a font was installed or
//! removed, the fonts are read again. No timer of its own: coming back to the app from Font Book
//! draws a frame, and that frame looks.
//!
//! On macOS a font manager or Font Book also switches fonts on and off without moving their files
//! (CoreText registers them from wherever they are), which the folders don't show. A background
//! thread looks at CoreText's font families twice a second and at its full list of font files
//! every five seconds ([`platform`]), and wakes the app when either changed and has settled: a
//! family switched on or off is in the font menus within about a second, a single style within a
//! few seconds, even
//! while DesignCraft is in the background. It asks rather than listens for CoreText's
//! notification because subscribing to that needs `unsafe` Objective-C calls, which the workspace
//! forbids.

use serde_json::json;

use crate::DesignApp;

/// Seconds between looks at the font folders.
const EVERY: f64 = 3.0;

#[derive(Clone, Copy, Default)]
struct Watch {
    next: f64,
    signature: Option<u64>,
}

/// Reloads at once when the platform's list of switched-on fonts changed (macOS; nothing to do
/// elsewhere). Runs on every logic tick, also while the window is hidden and before the UI's own
/// fonts have their installed faces: the watch wakes the app, and nothing else may.
pub(crate) fn platform_tick(app: &mut DesignApp, ctx: &egui::Context) {
    #[cfg(target_os = "macos")]
    {
        platform::start(ctx);
        if platform::take() {
            reload(app);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, ctx);
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
        reload(app);
    }
    changed
}

/// Read the fonts again and recompose the open documents (`font.reload`), saying so in the status
/// bar. Text set in a font that is gone keeps its font and shows as missing.
fn reload(app: &mut DesignApp) {
    match app.run("font.reload", json!({})) {
        Ok(_) => {
            let msg = crate::i18n::tr(&app.ui.language, "Fonts updated").to_owned();
            app.status(msg);
        }
        Err(e) => app.status(e),
    }
}

/// macOS: the watch on CoreText's list of available fonts.
#[cfg(target_os = "macos")]
mod platform {
    use std::sync::Once;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    /// How often the cheap look runs: CoreText's family names, a few milliseconds.
    const LOOK_EVERY: Duration = Duration::from_millis(500);
    /// Every this many cheap looks, the full list of font files (tens of milliseconds): it also
    /// sees a style switched on or off in a family that stays available.
    const FULL_EVERY: u32 = 10;
    /// How long the list must stay the same before the fonts are reloaded: a font manager
    /// switching a family on changes it in steps, and the batch is reloaded once.
    const SETTLE: Duration = Duration::from_millis(500);

    /// The list changed and settled since [`take`] last answered.
    static CHANGED: AtomicBool = AtomicBool::new(false);

    /// Start the watch (once); it wakes `ctx` when the list changed.
    pub(super) fn start(ctx: &egui::Context) {
        static START: Once = Once::new();
        START.call_once(|| {
            let wake = ctx.clone();
            // Without a thread to spare, coming back to the app still finds fonts in the font
            // folders, as before.
            let _ = std::thread::Builder::new().name("font-list-watch".into()).spawn(move || watch(&wake));
        });
    }

    /// Both looks: the families' and the files'.
    fn both() -> (u64, u64) {
        (designcraft_fonts::platform_families_signature(), designcraft_fonts::platform_fonts_signature())
    }

    fn watch(wake: &egui::Context) {
        let (mut families, mut files) = both();
        let mut looks = 0u32;
        loop {
            std::thread::sleep(LOOK_EVERY);
            looks = looks.wrapping_add(1);
            let changed = designcraft_fonts::platform_families_signature() != families
                || (looks % FULL_EVERY == 0 && designcraft_fonts::platform_fonts_signature() != files);
            if !changed {
                continue;
            }
            // Wait for the batch to finish: two full looks SETTLE apart that agree.
            let mut now = both();
            loop {
                std::thread::sleep(SETTLE);
                let again = both();
                if again == now {
                    break;
                }
                now = again;
            }
            (families, files) = now;
            log::debug!("the available fonts changed: reloading them");
            CHANGED.store(true, Ordering::Release);
            wake.request_repaint();
        }
    }

    /// Whether the list changed (and settled) since the last call.
    pub(super) fn take() -> bool {
        CHANGED.swap(false, Ordering::AcqRel)
    }
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
