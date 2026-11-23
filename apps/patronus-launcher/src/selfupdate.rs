//! The launcher updating itself from the releases of its own repository.
//!
//! The library's own self-update is built around ArtCraft Launcher (its folders, its app bundle and
//! a signed publisher), so this is a small one of our own: find a newer release, download the
//! matching `.exe` files with their published SHA-256 checked, and swap them in. A running
//! `.exe` can be renamed on Windows but not overwritten, so the old file is moved aside to
//! `<name>.exe.old` (removed on the next start) and the new one written in its place.

use std::path::{Path, PathBuf};

use craft_update::http::{Stop, Transfer};
use craft_update::{App, Asset, JobPhase, Release, install, version};

use crate::core::Launcher;

/// The repository the launcher's own releases are published in. Fixed at build time (from the
/// `PATRONUS_LAUNCHER_REPO` environment variable) so nothing at run time can redirect updates.
pub const LAUNCHER_REPO: &str = match option_env!("PATRONUS_LAUNCHER_REPO") {
    Some(repo) => repo,
    None => "darcon2k12345/designcraft",
};

/// The launcher as craft-update knows it (used to check where a download comes from).
pub static LAUNCHER: App = App::new("patronuslauncher", "Patronus Launcher", "Installs and updates the Patronus apps", LAUNCHER_REPO);

pub const GUI_EXE: &str = "patronus-launcher.exe";
pub const CLI_EXE: &str = "patronus-launcher-cli.exe";

/// A newer launcher that can be installed.
#[derive(Clone, Debug)]
pub struct SelfUpdate {
    pub version: String,
    release: Release,
    gui: Option<Asset>,
    cli: Option<Asset>,
}

/// The release file name of the window (`cli = false`) or console (`cli = true`) program.
pub fn asset_name(cli: bool, version: &str) -> String {
    format!("patronus-launcher{}-{version}-windows-x64.exe", if cli { "-cli" } else { "" })
}

/// Whether `release` is newer than `current` and has launcher files attached. Pure.
pub fn decide(current: &str, release: &Release) -> Option<SelfUpdate> {
    let latest = release.version.as_ref()?;
    let have = version::parse(current)?;
    if !version::is_newer(latest, &have) {
        return None;
    }
    let v = latest.to_string();
    let find = |cli: bool| release.assets.iter().find(|a| a.name == asset_name(cli, &v)).cloned();
    let (gui, cli) = (find(false), find(true));
    if gui.is_none() && cli.is_none() {
        return None;
    }
    Some(SelfUpdate { version: v, release: release.clone(), gui, cli })
}

fn old_name(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".old");
    PathBuf::from(name)
}

/// Puts `new` in place of `target`. The old file is moved aside first (the only way to replace a
/// running program on Windows) and put back if writing the new one fails.
pub fn replace_file(new: &Path, target: &Path) -> Result<(), String> {
    let old = old_name(target);
    let _ = std::fs::remove_file(&old);
    if target.exists() {
        std::fs::rename(target, &old).map_err(|e| {
            format!("couldn't move {} aside ({e}). If it is in a protected folder, move the launcher somewhere you can write to.", target.display())
        })?;
    }
    if let Err(e) = std::fs::copy(new, target) {
        let _ = std::fs::remove_file(target);
        if old.exists() {
            let _ = std::fs::rename(&old, target);
        }
        return Err(format!("couldn't write {} ({e})", target.display()));
    }
    let _ = std::fs::remove_file(new);
    Ok(())
}

/// Removes the `.old` files a previous update left next to the running program.
pub fn remove_leftovers() {
    let Ok(exe) = std::env::current_exe() else { return };
    let Some(dir) = exe.parent() else { return };
    for name in [GUI_EXE, CLI_EXE] {
        let _ = std::fs::remove_file(old_name(&dir.join(name)));
    }
    let _ = std::fs::remove_file(old_name(&exe));
}

impl Launcher {
    /// Looks for a newer launcher (network). `None` when this one is current, or the newest
    /// release has no launcher files attached.
    pub fn check_self(&self) -> Result<Option<SelfUpdate>, String> {
        Ok(self.latest_release(&LAUNCHER)?.and_then(|release| decide(env!("CARGO_PKG_VERSION"), &release)))
    }

    /// Downloads the new program files, checks them, and swaps them in. Returns the path of the
    /// running program, which is the new version once this returns.
    pub fn apply_self(&self, update: &SelfUpdate, phase: &mut dyn FnMut(JobPhase)) -> Result<PathBuf, String> {
        let exe = std::env::current_exe().map_err(|e| format!("couldn't tell where the launcher is ({e})"))?;
        let dir = exe.parent().ok_or("couldn't tell which folder the launcher is in")?.to_path_buf();
        let running_cli = exe.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.contains("cli"));
        let (own, other) = if running_cli { (update.cli.as_ref(), update.gui.as_ref()) } else { (update.gui.as_ref(), update.cli.as_ref()) };
        let mut targets: Vec<(PathBuf, &Asset)> = vec![(exe.clone(), own.ok_or("this release has no file for this program")?)];
        let sibling = dir.join(if running_cli { GUI_EXE } else { CLI_EXE });
        if let Some(asset) = other
            && sibling != exe
            && sibling.is_file()
        {
            targets.push((sibling, asset));
        }
        // Download everything first, so a failed download leaves the installed files untouched.
        let downloads = self.paths.downloads();
        std::fs::create_dir_all(&downloads).map_err(|e| format!("couldn't create {} ({e})", downloads.display()))?;
        let mut files = Vec::new();
        for (_, asset) in &targets {
            install::check_download_link(&LAUNCHER, &update.release, asset)?;
            let sha = asset.sha256.as_deref().ok_or_else(|| format!("{} has no published checksum, so it wasn't downloaded.", asset.name))?;
            let dest = downloads.join(&asset.name);
            let mut report = |t: Transfer| match t {
                Transfer::Bytes { done, total } => phase(JobPhase::Downloading { done, total }),
                Transfer::Reconnecting { done, total } => phase(JobPhase::Reconnecting { done, total }),
            };
            self.http.download(&asset.url, &dest, asset.size, sha, &Stop::default(), &mut report)?;
            files.push(dest);
        }
        phase(JobPhase::Installing);
        for ((target, _), file) in targets.iter().zip(&files) {
            replace_file(file, target)?;
        }
        Ok(exe)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn release(tag: &str, names: &[&str]) -> Release {
        let assets = names
            .iter()
            .map(|n| Asset {
                name: (*n).into(),
                size: 1,
                url: format!("https://github.com/o/r/releases/download/{tag}/{n}"),
                sha256: Some("a".repeat(64)),
            })
            .collect();
        Release { tag: tag.into(), version: version::parse(tag), assets, ..Default::default() }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("patronus-selfupdate-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_newer_release_with_launcher_files_is_offered() {
        let r = release("v1.0.1", &["patronus-launcher-1.0.1-windows-x64.exe", "patronus-launcher-cli-1.0.1-windows-x64.exe", "SHA256SUMS.txt"]);
        let update = decide("1.0.0", &r).expect("an update");
        assert_eq!(update.version, "1.0.1");
        assert!(update.gui.is_some() && update.cli.is_some());
    }

    #[test]
    fn the_same_or_an_older_release_is_not() {
        let r = release("v1.0.1", &["patronus-launcher-1.0.1-windows-x64.exe"]);
        assert!(decide("1.0.1", &r).is_none() && decide("2.0.0", &r).is_none());
    }

    #[test]
    fn a_release_without_launcher_files_is_not() {
        let r = release("v1.0.1", &["patronusdesign-1.0.1-windows-x64.msi", "patronus-launcher-9.9.9-windows-x64.exe"]);
        assert!(decide("1.0.0", &r).is_none(), "only the file for this exact version counts");
    }

    #[test]
    fn only_the_files_that_exist_are_offered() {
        let r = release("v1.0.1", &["patronus-launcher-cli-1.0.1-windows-x64.exe"]);
        let update = decide("1.0.0", &r).expect("an update");
        assert!(update.gui.is_none() && update.cli.is_some());
    }

    #[test]
    fn a_release_whose_version_cannot_be_read_is_not_offered() {
        let mut r = release("v1.0.1", &["patronus-launcher-1.0.1-windows-x64.exe"]);
        r.version = None;
        assert!(decide("1.0.0", &r).is_none());
    }

    #[test]
    fn asset_names_follow_the_release_workflow() {
        assert_eq!(asset_name(false, "0.21.2"), "patronus-launcher-0.21.2-windows-x64.exe");
        assert_eq!(asset_name(true, "0.21.2"), "patronus-launcher-cli-0.21.2-windows-x64.exe");
    }

    #[test]
    fn replacing_moves_the_old_file_aside_and_writes_the_new_one() {
        let dir = scratch("swap");
        let (target, new) = (dir.join("patronus-launcher.exe"), dir.join("download.exe"));
        std::fs::write(&target, "old").unwrap();
        std::fs::write(&new, "new").unwrap();
        replace_file(&new, &target).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(dir.join("patronus-launcher.exe.old")).unwrap(), "old");
        assert!(!new.exists(), "the download is consumed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_replacement_puts_the_old_file_back() {
        let dir = scratch("rollback");
        let target = dir.join("patronus-launcher.exe");
        std::fs::write(&target, "old").unwrap();
        let error = replace_file(&dir.join("missing.exe"), &target).unwrap_err();
        assert!(error.contains("couldn't write"), "{error}");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old", "nothing is lost");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replacing_works_when_there_is_nothing_to_replace_yet() {
        let dir = scratch("fresh");
        let (target, new) = (dir.join("patronus-launcher-cli.exe"), dir.join("download.exe"));
        std::fs::write(&new, "new").unwrap();
        replace_file(&new, &target).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
