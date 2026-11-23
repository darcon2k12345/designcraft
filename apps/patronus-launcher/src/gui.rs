//! Patronus Launcher: a small window to install, update and start Patronus Design, PdfCraft and
//! PhotoCraft. (The console version is `patronus-launcher-cli`.)
#![cfg_attr(windows, windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(not(windows))]
fn main() {
    eprintln!("The launcher window is for Windows. On this system use patronus-launcher-cli.");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() -> eframe::Result {
    window::run()
}

#[cfg(windows)]
mod window {
    use std::sync::Arc;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::thread;

    use craft_update::JobPhase;
    use eframe::egui::{self, Align, Color32, RichText};
    use launcher_core::{Action, Launcher, SelfUpdate, Tone, View};
    use patronus_release::APPS;

    /// Badge colours, in the order of `APPS`: Patronus Design, PdfCraft, PhotoCraft.
    const ACCENTS: [Color32; 3] = [Color32::from_rgb(0x4f, 0xc3, 0x6a), Color32::from_rgb(0xe0, 0x40, 0x44), Color32::from_rgb(0x3f, 0x91, 0xdf)];
    /// 96x96 raw RGBA app icons, in the order of `APPS`.
    const APP_ICONS: [&[u8]; 3] =
        [include_bytes!("../assets/design-96.rgba"), include_bytes!("../assets/pdf-96.rgba"), include_bytes!("../assets/photo-96.rgba")];
    /// 64x64 raw RGBA launcher icon, for the window.
    const WINDOW_ICON: &[u8] = include_bytes!("../assets/launcher-64.rgba");
    const GOOD: Color32 = Color32::from_rgb(110, 205, 130);
    const ATTENTION: Color32 = Color32::from_rgb(245, 190, 70);
    const BAD: Color32 = Color32::from_rgb(242, 100, 100);

    /// What the window asks the worker thread to do (app = index into `APPS`).
    enum Command {
        Refresh,
        Install(usize),
        Launch(usize),
        UpdateSelf,
    }

    /// What the worker thread reports back.
    enum Event {
        Viewed(usize, Result<View, String>),
        Progress(usize, String, Option<f32>),
        Finished(usize, Result<String, String>),
        Opened(usize, Result<(), String>),
        /// The launcher couldn't even start (no data folder, ...).
        Fatal(String),
        /// The newer launcher version, if there is one (or why the check failed).
        SelfChecked(Result<Option<String>, String>),
        SelfProgress(String, Option<f32>),
        /// The launcher was replaced and restarted (or why not).
        SelfDone(Result<(), String>),
    }

    /// Where the launcher's own update stands.
    enum SelfState {
        Unknown,
        Available(String),
        Working(String, Option<f32>),
        Failed(String),
        Restarting,
    }

    #[derive(Default)]
    struct Row {
        view: Option<Result<View, String>>,
        loading: bool,
        /// Label and, when known, the fraction done.
        progress: Option<(String, Option<f32>)>,
        /// The outcome of the last install/start, and whether it was an error.
        note: Option<(String, bool)>,
    }

    struct LauncherApp {
        rows: Vec<Row>,
        commands: Sender<Command>,
        events: Receiver<Event>,
        fatal: Option<String>,
        /// An install is running; every button waits for it.
        working: bool,
        icons: Vec<egui::TextureHandle>,
        self_state: SelfState,
        /// Set when the launcher has replaced itself and should close.
        close: bool,
    }

    fn phase_text(phase: JobPhase) -> (String, Option<f32>) {
        match phase {
            JobPhase::Downloading { done, total } if total > 0 => {
                (format!("Downloading\u{2026} {}%", done * 100 / total), Some(done as f32 / total as f32))
            }
            JobPhase::Downloading { .. } => ("Downloading\u{2026}".to_string(), None),
            JobPhase::Reconnecting { .. } => ("Connection dropped, retrying\u{2026}".to_string(), None),
            JobPhase::Verifying => ("Verifying the download\u{2026}".to_string(), None),
            JobPhase::Waiting => ("Waiting for the Windows installer (approve the prompt if one appears)\u{2026}".to_string(), None),
            JobPhase::Installing => ("Installing\u{2026}".to_string(), None),
            JobPhase::Removing => ("Removing the old version\u{2026}".to_string(), None),
        }
    }

    /// Owns the `Launcher` and does all the slow work, so the window never freezes.
    fn worker(ctx: egui::Context, commands: Receiver<Command>, events: Sender<Event>) {
        let send = |event: Event| {
            let _ = events.send(event);
            ctx.request_repaint();
        };
        let mut launcher = match Launcher::open() {
            Ok(launcher) => launcher,
            Err(message) => return send(Event::Fatal(message)),
        };
        let mut pending: Option<SelfUpdate> = None;
        while let Ok(command) = commands.recv() {
            match command {
                Command::Refresh => {
                    for (i, app) in APPS.iter().enumerate() {
                        send(Event::Viewed(i, launcher.view(app)));
                    }
                    let checked = launcher.check_self();
                    pending = checked.as_ref().ok().cloned().flatten();
                    send(Event::SelfChecked(checked.map(|found| found.map(|u| u.version))));
                }
                Command::Install(i) => {
                    let Some(app) = APPS.get(i) else { continue };
                    let mut phase = |p: JobPhase| {
                        let (label, fraction) = phase_text(p);
                        send(Event::Progress(i, label, fraction));
                    };
                    let outcome = launcher.install(app, &mut phase);
                    send(Event::Finished(i, outcome));
                    send(Event::Viewed(i, launcher.view(app)));
                }
                Command::Launch(i) => {
                    let Some(app) = APPS.get(i) else { continue };
                    send(Event::Opened(i, launcher.launch(app)));
                }
                Command::UpdateSelf => {
                    let Some(update) = pending.take() else { continue };
                    let mut phase = |p: JobPhase| {
                        let (label, fraction) = phase_text(p);
                        send(Event::SelfProgress(label, fraction));
                    };
                    let outcome = launcher.apply_self(&update, &mut phase).and_then(|exe| {
                        std::process::Command::new(&exe)
                            .spawn()
                            .map(|_| ())
                            .map_err(|e| format!("the launcher was updated but couldn't be restarted ({e}). Start it again by hand."))
                    });
                    send(Event::SelfDone(outcome));
                }
            }
        }
    }

    impl LauncherApp {
        fn new(ctx: egui::Context) -> Self {
            let ctx_for_textures = ctx.clone();
            let (commands, command_rx) = channel();
            let (event_tx, events) = channel();
            thread::spawn(move || worker(ctx, command_rx, event_tx));
            let icons = APP_ICONS
                .iter()
                .enumerate()
                .map(|(i, bytes)| {
                    let image = egui::ColorImage::from_rgba_unmultiplied([96, 96], bytes);
                    ctx_for_textures.load_texture(format!("app-icon-{i}"), image, egui::TextureOptions::LINEAR)
                })
                .collect();
            let mut app = Self {
                rows: APPS.iter().map(|_| Row::default()).collect(),
                commands,
                events,
                fatal: None,
                working: false,
                icons,
                self_state: SelfState::Unknown,
                close: false,
            };
            app.refresh();
            app
        }

        fn refresh(&mut self) {
            for row in &mut self.rows {
                row.loading = true;
            }
            let _ = self.commands.send(Command::Refresh);
        }

        fn dispatch(&mut self, command: Command) {
            match &command {
                Command::Install(i) => {
                    self.working = true;
                    if let Some(row) = self.rows.get_mut(*i) {
                        row.note = None;
                        row.progress = Some(("Starting\u{2026}".to_string(), None));
                    }
                }
                Command::Launch(i) => {
                    if let Some(row) = self.rows.get_mut(*i) {
                        row.note = None;
                    }
                }
                Command::UpdateSelf => {
                    self.working = true;
                    self.self_state = SelfState::Working("Starting\u{2026}".to_string(), None);
                }
                Command::Refresh => {}
            }
            let _ = self.commands.send(command);
        }

        fn drain_events(&mut self) {
            while let Ok(event) = self.events.try_recv() {
                match event {
                    Event::Viewed(i, view) => {
                        if let Some(row) = self.rows.get_mut(i) {
                            row.loading = false;
                            row.view = Some(view);
                        }
                    }
                    Event::Progress(i, label, fraction) => {
                        if let Some(row) = self.rows.get_mut(i) {
                            row.progress = Some((label, fraction));
                        }
                    }
                    Event::Finished(i, outcome) => {
                        self.working = false;
                        if let Some(row) = self.rows.get_mut(i) {
                            row.progress = None;
                            row.note = Some(match outcome {
                                Ok(message) => (message, false),
                                Err(message) => (message, true),
                            });
                        }
                    }
                    Event::Opened(i, outcome) => {
                        if let Some(row) = self.rows.get_mut(i) {
                            row.note = Some(match outcome {
                                Ok(()) => (format!("Started {}.", APPS.get(i).map_or("the app", |a| a.name)), false),
                                Err(message) => (message, true),
                            });
                        }
                    }
                    Event::SelfChecked(result) => {
                        self.self_state = match result {
                            Ok(Some(version)) => SelfState::Available(version),
                            // Not being able to check is not worth a banner: the apps still work.
                            Ok(None) | Err(_) => SelfState::Unknown,
                        };
                    }
                    Event::SelfProgress(label, fraction) => self.self_state = SelfState::Working(label, fraction),
                    Event::SelfDone(outcome) => {
                        self.working = false;
                        match outcome {
                            Ok(()) => {
                                self.self_state = SelfState::Restarting;
                                self.close = true;
                            }
                            Err(message) => self.self_state = SelfState::Failed(message),
                        }
                    }
                    Event::Fatal(message) => {
                        self.fatal = Some(message);
                        for row in &mut self.rows {
                            row.loading = false;
                        }
                    }
                }
            }
        }

        fn card(&mut self, ui: &mut egui::Ui, i: usize) {
            let (Some(app), Some(row)) = (APPS.get(i), self.rows.get(i)) else { return };
            let accent = ACCENTS.get(i).copied().unwrap_or(Color32::GRAY);
            let working = self.working;
            let mut clicked = None;
            egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(10.0).inner_margin(egui::Margin::same(14)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let size = egui::vec2(46.0, 46.0);
                    if let Some(icon) = self.icons.get(i) {
                        ui.add(egui::Image::new((icon.id(), size)));
                    } else {
                        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 10.0, accent);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(app.name).size(19.0).strong());
                        ui.label(RichText::new(app.tagline).weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let view = row.view.as_ref().and_then(|v| v.as_ref().ok());
                        let primary = match view.map(|v| v.action) {
                            Some(Action::Install) => Some("Install"),
                            Some(Action::Update) => Some("Update"),
                            _ => None,
                        };
                        if let Some(label) = primary {
                            let button =
                                egui::Button::new(RichText::new(label).color(Color32::WHITE).strong()).fill(accent).min_size(egui::vec2(96.0, 32.0));
                            if ui.add_enabled(!working, button).clicked() {
                                clicked = Some(Command::Install(i));
                            }
                        }
                        if view.is_some_and(|v| v.can_launch)
                            && ui.add_enabled(!working, egui::Button::new("Open").min_size(egui::vec2(72.0, 32.0))).clicked()
                        {
                            clicked = Some(Command::Launch(i));
                        }
                    });
                });
                ui.add_space(6.0);
                match &row.view {
                    None if row.loading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new(format!("Checking github.com/{}\u{2026}", app.repo)).weak());
                        });
                    }
                    None => {}
                    Some(Err(message)) => {
                        ui.label(RichText::new(format!("Couldn't check for updates: {message}")).color(BAD));
                    }
                    Some(Ok(view)) => {
                        let installed = view.installed.as_deref().map_or("not installed".to_string(), |v| format!("version {v}"));
                        let latest = view.latest.as_deref().map_or("no release yet".to_string(), |v| format!("version {v}"));
                        ui.label(RichText::new(format!("Installed: {installed}    Latest: {latest}")).weak());
                        let colour = match view.tone {
                            Tone::Good => GOOD,
                            Tone::Attention => ATTENTION,
                            Tone::Neutral => ui.visuals().text_color(),
                        };
                        ui.label(RichText::new(&view.summary).color(colour));
                    }
                }
                if let Some((label, fraction)) = &row.progress {
                    ui.add_space(4.0);
                    match fraction {
                        Some(f) => {
                            ui.add(egui::ProgressBar::new(*f).text(label.as_str()));
                        }
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(label.as_str());
                            });
                        }
                    }
                }
                if let Some((message, is_error)) = &row.note {
                    ui.add_space(4.0);
                    ui.label(RichText::new(message).color(if *is_error { BAD } else { GOOD }));
                }
                ui.add_space(2.0);
                ui.hyperlink_to(RichText::new("Releases on GitHub").small(), format!("https://github.com/{}/releases", app.repo));
            });
            if let Some(command) = clicked {
                self.dispatch(command);
            }
        }
    }

    impl eframe::App for LauncherApp {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.drain_events();
            if self.close {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            egui::Frame::new().inner_margin(egui::Margin::same(18)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Patronus Launcher");
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let busy = self.working || self.rows.iter().any(|r| r.loading);
                        if ui.add_enabled(!busy, egui::Button::new("Refresh")).clicked() {
                            self.refresh();
                        }
                    });
                });
                ui.label(RichText::new("Install and update Patronus Design, PdfCraft and PhotoCraft.").weak());
                ui.add_space(10.0);
                let mut update_clicked = false;
                match &self.self_state {
                    SelfState::Available(version) => {
                        egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(8.0).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("A new version of Patronus Launcher is available: {version}.")).color(ATTENTION));
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    if ui.add_enabled(!self.working, egui::Button::new("Update launcher")).clicked() {
                                        update_clicked = true;
                                    }
                                });
                            });
                        });
                        ui.add_space(8.0);
                    }
                    SelfState::Working(label, fraction) => {
                        match fraction {
                            Some(f) => {
                                ui.add(egui::ProgressBar::new(*f).text(format!("Updating the launcher: {label}")));
                            }
                            None => {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(format!("Updating the launcher: {label}"));
                                });
                            }
                        }
                        ui.add_space(8.0);
                    }
                    SelfState::Failed(message) => {
                        ui.label(RichText::new(format!("The launcher couldn't update itself: {message}")).color(BAD));
                        ui.add_space(8.0);
                    }
                    SelfState::Restarting => {
                        ui.label(RichText::new("Updated. Restarting\u{2026}").color(GOOD));
                        ui.add_space(8.0);
                    }
                    SelfState::Unknown => {}
                }
                if update_clicked {
                    self.dispatch(Command::UpdateSelf);
                }
                if let Some(message) = &self.fatal {
                    ui.label(RichText::new(format!("The launcher couldn't start: {message}")).color(BAD));
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for i in 0..self.rows.len() {
                        self.card(ui, i);
                        ui.add_space(10.0);
                    }
                });
            });
        }
    }

    pub fn run() -> eframe::Result {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Patronus Launcher")
                .with_icon(Arc::new(egui::IconData { rgba: WINDOW_ICON.to_vec(), width: 64, height: 64 }))
                .with_inner_size([720.0, 600.0])
                .with_min_inner_size([520.0, 420.0]),
            ..Default::default()
        };
        eframe::run_native(
            "Patronus Launcher",
            options,
            Box::new(|cc| {
                cc.egui_ctx.set_visuals(egui::Visuals::dark());
                Ok(Box::new(LauncherApp::new(cc.egui_ctx.clone())))
            }),
        )
    }
}
