//! What the launcher should do about the latest release. Pure: no network, no disk.

use craft_update::{App, Asset, Installed, PackageKind, Platform, Release, install, pick_asset, version};

/// The outcome of comparing what is installed with the latest release.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan<'a> {
    /// What is installed is already the latest (or newer).
    UpToDate { version: String },
    /// Download `asset` and install it as `kind`. `from` is the installed version, if any.
    Install { asset: &'a Asset, kind: PackageKind, from: Option<String> },
    /// The release has nothing for this system's package formats.
    NoPackage,
    /// Something else owns the installed copy; say how to update it there.
    ManagedElsewhere(String),
}

/// Decides what to do. `kinds` are the package formats to try, best first, as
/// `install::choose_kinds` returns them (empty when the launcher must not touch the install).
pub fn decide<'a>(app: &App, release: &'a Release, installed: Option<&Installed>, kinds: &[PackageKind], platform: Platform) -> Plan<'a> {
    if let Some(inst) = installed
        && let (Some(latest), Some(have)) = (&release.version, &inst.version)
        && !version::is_newer(latest, have)
    {
        return Plan::UpToDate { version: have.to_string() };
    }
    if kinds.is_empty() {
        let message = installed.and_then(|i| install::managed_elsewhere(app, i, false)).unwrap_or_else(|| {
            let how = installed.map_or("another way", |i| i.source.label());
            format!("{} is installed through {how}, which this launcher doesn't manage. Update it the way it was installed.", app.name)
        });
        return Plan::ManagedElsewhere(message);
    }
    let stems: Vec<&str> = app.stems().collect();
    match pick_asset(release, &stems, platform, kinds) {
        Some((asset, kind)) => Plan::Install { asset, kind, from: installed.and_then(|i| i.version.as_ref().map(ToString::to_string)) },
        None => Plan::NoPackage,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use craft_update::{Arch, InstallSource, Os};
    use patronus_release::APP;

    const LINUX: Platform = Platform { os: Os::Linux, arch: Arch::X64 };

    fn asset(name: &str) -> Asset {
        Asset { name: name.into(), size: 1, url: format!("https://github.com/x/y/releases/download/v0.21.0/{name}"), sha256: Some("a".repeat(64)) }
    }

    fn release(names: &[&str]) -> Release {
        Release { tag: "v0.21.0".into(), version: version::parse("0.21.0"), assets: names.iter().map(|n| asset(n)).collect(), ..Default::default() }
    }

    fn installed(v: &str, source: InstallSource) -> Installed {
        Installed {
            version: version::parse(v),
            source,
            location: "/somewhere".into(),
            package: None,
            externally_managed: false,
            origin: None,
            installation: None,
        }
    }

    const ALL: [PackageKind; 3] = [PackageKind::AppImage, PackageKind::Deb, PackageKind::TarGz];

    #[test]
    fn fresh_install_takes_the_best_available_format() {
        let r = release(&["patronusdesign-0.21.0-linux-x86_64.tar.gz", "patronusdesign-0.21.0-linux-x86_64.deb"]);
        let Plan::Install { asset, kind, from } = decide(&APP, &r, None, &ALL, LINUX) else { panic!("expected an install") };
        assert_eq!((kind, from, asset.name.as_str()), (PackageKind::Deb, None, "patronusdesign-0.21.0-linux-x86_64.deb"));
    }

    #[test]
    fn older_install_is_updated_and_reports_where_from() {
        let r = release(&["patronusdesign-0.21.0-linux-x86_64.deb"]);
        let inst = installed("0.20.1", InstallSource::Deb);
        let Plan::Install { from, .. } = decide(&APP, &r, Some(&inst), &[PackageKind::Deb], LINUX) else { panic!("expected an install") };
        assert_eq!(from.as_deref(), Some("0.20.1"));
    }

    #[test]
    fn same_or_newer_install_is_up_to_date() {
        let r = release(&["patronusdesign-0.21.0-linux-x86_64.deb"]);
        for have in ["0.21.0", "0.22.0"] {
            let inst = installed(have, InstallSource::Launcher);
            assert_eq!(decide(&APP, &r, Some(&inst), &ALL, LINUX), Plan::UpToDate { version: have.into() });
        }
    }

    #[test]
    fn releases_still_using_the_old_designcraft_names_are_found() {
        let r = release(&["designcraft-0.21.0-linux-x86_64.deb"]);
        assert!(matches!(decide(&APP, &r, None, &ALL, LINUX), Plan::Install { kind: PackageKind::Deb, .. }));
    }

    #[test]
    fn other_apps_and_other_platforms_are_ignored() {
        let others =
            release(&["photocraft-0.21.0-linux-x86_64.deb", "patronusdesign-0.21.0-windows-x64.msi", "patronusdesign-0.21.0-macos-universal.dmg"]);
        assert_eq!(decide(&APP, &others, None, &ALL, LINUX), Plan::NoPackage);
    }

    #[test]
    fn an_install_the_launcher_must_not_touch_is_left_alone() {
        let r = release(&["patronusdesign-0.21.0-linux-x86_64.deb"]);
        let inst = installed("0.20.1", InstallSource::Binary);
        let Plan::ManagedElsewhere(message) = decide(&APP, &r, Some(&inst), &[], LINUX) else { panic!("expected managed elsewhere") };
        assert!(message.contains("outside the launcher"), "{message}");
    }
}
