//! Looking up releases, deciding what to do, installing and starting apps.

use std::path::PathBuf;

use craft_update::detect::{Snapshot, detect};
use craft_update::install::{self, Job};
use craft_update::manifest::Manifest;
use craft_update::source::{GitHubSource, ReleaseSource};
use craft_update::{
    App, Asset, Http, InstallLocations, InstallSource, Installed, JobPhase, LatestRelease, PackageKind, Paths, Platform, Release, Stop,
};

use crate::plan::{self, Plan};

/// What the app's main button should do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Install,
    Update,
    Nothing,
}

/// How a status line should be coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Good,
    Attention,
    Neutral,
}

/// Everything a screen needs to show about one app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// The installed version ("0.21.0"), if the app is installed.
    pub installed: Option<String>,
    /// The latest published version, if there is a release.
    pub latest: Option<String>,
    /// One sentence on where things stand.
    pub summary: String,
    pub tone: Tone,
    pub action: Action,
    /// Whether the app can be started.
    pub can_launch: bool,
}

pub struct Launcher {
    paths: Paths,
    http: Http,
    source: GitHubSource,
    manifest_path: PathBuf,
    manifest: Manifest,
}

/// "0.21.0 (Installer (.msi), C:\\Program Files\\…)" for the console tool.
pub fn describe(installed: Option<&Installed>) -> String {
    match installed {
        None => "not installed".to_string(),
        Some(i) => {
            let version = i.version.as_ref().map_or_else(|| "unknown version".to_string(), ToString::to_string);
            // craft-update's own label for launcher-managed copies names ArtCraft Launcher.
            let how = if i.source == InstallSource::Launcher { "Patronus Launcher" } else { i.source.label() };
            format!("{version} ({how}, {})", i.location)
        }
    }
}

impl Launcher {
    pub fn open() -> Result<Self, String> {
        let dir = if cfg!(any(target_os = "macos", target_os = "windows")) { "Patronus Launcher" } else { "patronus-launcher" };
        let paths = Paths::per_user(dir)?;
        for folder in [&paths.data, &paths.cache] {
            std::fs::create_dir_all(folder).map_err(|e| format!("couldn't create {} ({e})", folder.display()))?;
        }
        let agent = format!("Patronus-Launcher/{}", env!("CARGO_PKG_VERSION"));
        let token = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty());
        let http = Http::new(&agent)?.with_github_token(token);
        let source = GitHubSource::new(http.clone(), paths.cache.join("releases.json"));
        let manifest_path = paths.data.join("installed.json");
        let manifest = Manifest::load(&manifest_path);
        Ok(Self { paths, http, source, manifest_path, manifest })
    }

    pub fn installed_copy(&self, app: &App) -> Option<Installed> {
        detect(app, &self.manifest, &Snapshot::take())
    }

    pub fn latest_release(&self, app: &App) -> Result<Option<Release>, String> {
        let answer = self.source.latest(app.repo, false);
        self.source.save();
        match answer.latest {
            LatestRelease::Found(release) => Ok(Some(release)),
            LatestRelease::None => Ok(None),
            LatestRelease::Failed(why) => Err(format!("couldn't look up the latest release of {} ({why})", app.repo)),
        }
    }

    pub fn plan<'a>(&self, app: &App, release: &'a Release, installed: Option<&Installed>) -> Result<Plan<'a>, String> {
        let platform = Platform::current().ok_or("this system isn't supported")?;
        let record = self.manifest.apps.get(app.id);
        let kinds = install::choose_kinds(app, installed, record, platform.default_kinds());
        Ok(plan::decide(app, release, installed, &kinds, platform))
    }

    /// Looks up the latest release (network) and works out what the screen should say and offer.
    pub fn view(&self, app: &App) -> Result<View, String> {
        let installed = self.installed_copy(app);
        let installed_version = installed.as_ref().map(|i| i.version.as_ref().map_or_else(|| "unknown".to_string(), ToString::to_string));
        let can_launch = installed.is_some();
        let Some(release) = self.latest_release(app)? else {
            return Ok(View {
                installed: installed_version,
                latest: None,
                summary: format!("No release has been published on github.com/{} yet.", app.repo),
                tone: Tone::Neutral,
                action: Action::Nothing,
                can_launch,
            });
        };
        let latest = release.version_label();
        let (summary, tone, action) = match self.plan(app, &release, installed.as_ref())? {
            Plan::UpToDate { .. } => ("Up to date.".to_string(), Tone::Good, Action::Nothing),
            Plan::Install { from: None, .. } => (format!("Version {latest} is ready to install."), Tone::Attention, Action::Install),
            Plan::Install { from: Some(have), .. } => (format!("Update available: {have} to {latest}."), Tone::Attention, Action::Update),
            Plan::NoPackage => (
                format!("Release {latest} has no installer for this system attached to it. Publish the Windows .msi and .zip with the release."),
                Tone::Neutral,
                Action::Nothing,
            ),
            Plan::ManagedElsewhere(message) => (message, Tone::Neutral, Action::Nothing),
        };
        Ok(View { installed: installed_version, latest: Some(latest), summary, tone, action, can_launch })
    }

    /// Downloads, verifies and installs a chosen asset, then records it.
    pub fn run_install(
        &mut self,
        app: &App,
        release: &Release,
        asset: &Asset,
        kind: PackageKind,
        installed: Option<&Installed>,
        phase: &mut dyn FnMut(JobPhase),
    ) -> Result<(), String> {
        let (locations, stop) = (InstallLocations::default(), Stop::default());
        let previous = self.manifest.apps.get(app.id).cloned();
        let record = {
            let mut job = Job { http: &self.http, paths: &self.paths, stop: &stop, phase, install_locations: &locations, replace: false };
            install::install(app, release, asset, kind, installed, previous.as_ref(), &mut job)?
        };
        if let Some(record) = record {
            self.manifest.apps.insert(app.id.to_string(), record);
            self.manifest.save(&self.manifest_path)?;
        }
        Ok(())
    }

    /// Installs or updates to the latest release. Returns a sentence saying what happened.
    pub fn install(&mut self, app: &App, phase: &mut dyn FnMut(JobPhase)) -> Result<String, String> {
        let installed = self.installed_copy(app);
        let release = self.latest_release(app)?.ok_or_else(|| format!("{} has no published release yet", app.repo))?;
        match self.plan(app, &release, installed.as_ref())? {
            Plan::UpToDate { version } => Ok(format!("{} {version} is already the latest.", app.name)),
            Plan::NoPackage => Err(format!("{}: this release has no package for this system", app.name)),
            Plan::ManagedElsewhere(message) => Err(message),
            Plan::Install { asset, kind, from } => {
                self.run_install(app, &release, asset, kind, installed.as_ref(), phase)?;
                Ok(match from {
                    Some(v) => format!("Updated {} {v} to {}.", app.name, release.version_label()),
                    None => format!("Installed {} {}.", app.name, release.version_label()),
                })
            }
        }
    }

    pub fn launch(&self, app: &App) -> Result<(), String> {
        let installed = self.installed_copy(app).ok_or_else(|| format!("{} isn't installed.", app.name))?;
        install::launch(app, &installed, &self.paths)
    }
}
