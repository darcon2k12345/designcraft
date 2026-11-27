//! `patronus-launcher-cli`: install and update Patronus Design, Patronus PDF and Patronus Photo from the
//! console. The window version is `patronus-launcher`.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

/// `println!` that ignores a closed stdout (`patronus-launcher-cli status | head`) instead of panicking.
macro_rules! say {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stdout(), $($arg)*);
    }};
}

use std::process::ExitCode;

use craft_update::{App, JobPhase};
use launcher_core::plan::Plan;
use launcher_core::{Launcher, describe};
use patronus_release::APPS;

const USAGE: &str = "\
patronus-launcher-cli: install and update Patronus Design, Patronus PDF and Patronus Photo

USAGE:
    patronus-launcher-cli <command> [app]

COMMANDS:
    status [app]    What is installed, the latest release, and what an update would do (default; all apps)
    check [app]     Like status; exit 0 if everything is up to date, 10 if an install/update is available
    install <app>   Install the app, or update it if an older version is installed
    update [app]    Update installed apps (all of them if none is named); skips apps that aren't installed
    launch <app>    Start the installed app
    apps            List the apps this launcher manages
    self-update     Update this launcher itself to the newest release
    help            Show this help

<app> is one of: patronusdesign (also designcraft), patronuspdf (also pdfcraft, printcraft), patronusphoto (also photocraft).
Set GITHUB_TOKEN to raise GitHub's API rate limit. For the window version, run patronus-launcher.";

/// Exit status of `check` when there is something to install.
const UPDATE_AVAILABLE: u8 = 10;

/// The apps a command applies to: all of them, or the one named.
fn select(selector: Option<&str>) -> Result<Vec<&'static App>, String> {
    match selector {
        None | Some("all") => Ok(APPS.iter().collect()),
        Some(key) => patronus_release::find(key).map(|app| vec![app]).ok_or_else(|| {
            let ids: Vec<_> = APPS.iter().map(|a| a.id).collect();
            format!("unknown app `{key}` (choose one of: {})", ids.join(", "))
        }),
    }
}

/// The single app a command needs.
fn one(selector: Option<&str>, command: &str) -> Result<&'static App, String> {
    match selector {
        None | Some("all") => Err(format!("`{command}` needs an app, for example `patronus-launcher-cli {command} patronusdesign`")),
        Some(_) => select(selector).and_then(|apps| apps.first().copied().ok_or_else(|| "no such app".to_string())),
    }
}

/// Prints one app's state; returns `UPDATE_AVAILABLE` when something can be installed.
fn status(launcher: &Launcher, app: &App) -> Result<u8, String> {
    let installed = launcher.installed_copy(app);
    say!("{} from github.com/{}", app.name, app.repo);
    say!("  installed: {}", describe(installed.as_ref()));
    let Some(release) = launcher.latest_release(app)? else {
        say!("  latest:    no release published yet");
        return Ok(0);
    };
    say!("  latest:    {}", release.version_label());
    let (line, code) = match launcher.plan(app, &release, installed.as_ref())? {
        Plan::UpToDate { .. } => ("up to date".to_string(), 0),
        Plan::Install { asset, kind, from: None } => (format!("can install {} ({})", asset.name, kind.label()), UPDATE_AVAILABLE),
        Plan::Install { asset, kind, from: Some(_) } => (format!("update available: {} ({})", asset.name, kind.label()), UPDATE_AVAILABLE),
        Plan::NoPackage => ("this release has no package for this system".to_string(), 0),
        Plan::ManagedElsewhere(message) => (message, 0),
    };
    say!("  next:      {line}");
    Ok(code)
}

/// `status` for several apps. One app failing doesn't stop the others; any failure makes the
/// exit status 1, otherwise 10 when anything is installable and 0 when all is up to date.
fn status_all(launcher: &Launcher, apps: &[&App]) -> u8 {
    let (mut available, mut failed) = (false, false);
    for (i, app) in apps.iter().enumerate() {
        if i > 0 {
            say!();
        }
        match status(launcher, app) {
            Ok(code) => available |= code == UPDATE_AVAILABLE,
            Err(message) => {
                failed = true;
                eprintln!("  error: {message}");
            }
        }
    }
    if failed {
        1
    } else if available {
        UPDATE_AVAILABLE
    } else {
        0
    }
}

fn progress(phase: JobPhase, last: &mut Option<String>) {
    let text = match phase {
        JobPhase::Downloading { done, total } if total > 0 => format!("downloading {}%", done * 100 / total / 5 * 5),
        JobPhase::Downloading { .. } => "downloading".to_string(),
        JobPhase::Reconnecting { .. } => "connection dropped, retrying".to_string(),
        JobPhase::Verifying => "verifying checksum".to_string(),
        JobPhase::Waiting => "waiting for the system installer".to_string(),
        JobPhase::Installing => "installing".to_string(),
        JobPhase::Removing => "removing the old version".to_string(),
    };
    if last.as_deref() != Some(text.as_str()) {
        eprintln!("  {text}");
        *last = Some(text);
    }
}

fn install_or_update(launcher: &mut Launcher, app: &App, only_if_installed: bool) -> Result<u8, String> {
    if only_if_installed && launcher.installed_copy(app).is_none() {
        return Err(format!("{} isn't installed. Run `patronus-launcher-cli install {}` first.", app.name, app.id));
    }
    let mut last = None;
    let mut phase = |p: JobPhase| progress(p, &mut last);
    let message = launcher.install(app, &mut phase)?;
    say!("{message}");
    Ok(0)
}

/// `update` with no app named: update every installed app, carrying on past a failure.
fn update_all(launcher: &mut Launcher, apps: &[&App]) -> u8 {
    let (mut failed, mut any) = (false, false);
    for app in apps {
        if launcher.installed_copy(app).is_none() {
            continue;
        }
        any = true;
        if let Err(message) = install_or_update(launcher, app, true) {
            failed = true;
            eprintln!("error: {message}");
        }
    }
    if !any {
        say!("Nothing is installed yet. Use `patronus-launcher-cli install <app>`.");
    }
    u8::from(failed)
}

/// `self-update`: look for a newer launcher and install it.
fn self_update(launcher: &Launcher) -> Result<u8, String> {
    let current = env!("CARGO_PKG_VERSION");
    let Some(update) = launcher.check_self()? else {
        say!("The launcher is up to date ({current}).");
        return Ok(0);
    };
    say!("Updating the launcher {current} -> {}", update.version);
    let mut last = None;
    let mut phase = |p: JobPhase| progress(p, &mut last);
    launcher.apply_self(&update, &mut phase)?;
    say!("Done. Run the launcher again to use version {}.", update.version);
    Ok(0)
}

fn run(args: &[String]) -> Result<u8, String> {
    let selector = args.get(1).map(String::as_str);
    match args.first().map(String::as_str).unwrap_or("status") {
        "help" | "--help" | "-h" => {
            say!("{USAGE}");
            Ok(0)
        }
        "--version" | "-V" => {
            say!("patronus-launcher-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        "apps" => {
            for app in APPS.iter() {
                say!("{:<11} {}  (github.com/{})", app.id, app.name, app.repo);
            }
            Ok(0)
        }
        "status" | "check" => {
            let apps = select(selector)?;
            let launcher = Launcher::open()?;
            let code = status_all(&launcher, &apps);
            if selector.is_none() || selector == Some("all") {
                match launcher.check_self() {
                    Ok(Some(update)) => {
                        say!("\nThe launcher itself has an update: {} -> {} (run `self-update`).", env!("CARGO_PKG_VERSION"), update.version)
                    }
                    Ok(None) => say!("\nThe launcher is up to date ({}).", env!("CARGO_PKG_VERSION")),
                    Err(message) => eprintln!("\ncouldn't check for a launcher update: {message}"),
                }
            }
            Ok(code)
        }
        "self-update" => self_update(&Launcher::open()?),
        "install" => install_or_update(&mut Launcher::open()?, one(selector, "install")?, false),
        "update" => match selector {
            None | Some("all") => Ok(update_all(&mut Launcher::open()?, &select(selector)?)),
            Some(_) => install_or_update(&mut Launcher::open()?, one(selector, "update")?, true),
        },
        "launch" => {
            Launcher::open()?.launch(one(selector, "launch")?)?;
            Ok(0)
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}
