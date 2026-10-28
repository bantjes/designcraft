//! `cargo xtask bundle`: build the release app and wrap it as `dist/Marian DesignCraft.app`
//! (macOS), with the icon `assets/app-icon/marian-designcraft.icns` and the `Info.plist` that
//! `packaging/macos/package.sh` fills in the same way. Signed ad hoc when `codesign` is there, so
//! the Mac runs it from this folder.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The app's name: the bundle, its executable and its icon file.
pub const APP_NAME: &str = "Marian DesignCraft";

pub fn run(root: &Path) -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .env("MACOSX_DEPLOYMENT_TARGET", "11.0")
        .args(["build", "--release", "-p", "designcraft"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("release build failed".into());
    }
    let target = std::env::var("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|_| root.join("target"));
    let bundle = root.join("dist").join(format!("{APP_NAME}.app"));
    let _ = std::fs::remove_dir_all(&bundle);
    let contents = bundle.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(contents.join("Resources")).map_err(|e| e.to_string())?;
    std::fs::copy(target.join("release/designcraft"), contents.join("MacOS").join(APP_NAME)).map_err(|e| format!("copy app: {e}"))?;
    std::fs::copy(root.join("assets/app-icon/marian-designcraft.icns"), contents.join("Resources").join(format!("{APP_NAME}.icns")))
        .map_err(|e| format!("copy icon: {e}"))?;
    std::fs::copy(root.join("NOTICE"), contents.join("Resources").join("NOTICE")).map_err(|e| format!("copy NOTICE: {e}"))?;
    let sha = std::env::var("DESIGNCRAFT_BUILD_SHA").unwrap_or_else(|_| "unknown".into());
    std::fs::write(contents.join("Info.plist"), info_plist(env!("CARGO_PKG_VERSION"), &sha)).map_err(|e| e.to_string())?;
    std::fs::write(contents.join("PkgInfo"), "APPL????").map_err(|e| e.to_string())?;
    // An ad-hoc signature, as `package.sh` gives a local build; without `codesign` (not a Mac) the
    // bundle is still written.
    match Command::new("codesign").args(["--force", "--deep", "--sign", "-"]).arg(&bundle).status() {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("codesign failed ({s}); the app is unsigned"),
        Err(_) => eprintln!("no codesign here; the app is unsigned"),
    }
    println!("built {}", bundle.display());
    Ok(())
}

/// The release `Info.plist` template.
const INFO_PLIST: &str = include_str!("../../packaging/macos/Info.plist.in");

/// The bundle's `Info.plist`: [`INFO_PLIST`] for `version` and the build commit `sha`.
fn info_plist(version: &str, sha: &str) -> String {
    let short = version.split('-').next().unwrap_or(version);
    INFO_PLIST.replace("@VERSION@", version).replace("@SHORT_VERSION@", short).replace("@BUILD_SHA@", sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundle_is_marian_designcraft() {
        let p = info_plist("1.2.3-rc.1", "abc123");
        assert!(!p.contains('@'), "every placeholder filled");
        assert!(p.contains("<string>1.2.3</string>") && p.contains("<string>abc123</string>"));
        for (key, value) in [
            ("CFBundleName", APP_NAME),
            ("CFBundleDisplayName", APP_NAME),
            ("CFBundleExecutable", APP_NAME),
            ("CFBundleIconFile", APP_NAME),
            ("CFBundleIdentifier", "ai.storyteller.designcraft.marian"),
        ] {
            assert!(p.contains(&format!("<key>{key}</key>\n  <string>{value}</string>")), "{key}");
        }
        assert!(crate::root().join("assets/app-icon/marian-designcraft.icns").is_file());
    }
}
