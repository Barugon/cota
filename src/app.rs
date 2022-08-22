use crate::{
  about_dlg::AboutDlg,
  chronometer::Chronometer,
  config,
  confirm_dlg::{Choice, ConfirmDlg, Hence},
  experience::Experience,
  offline::Offline,
  stats::{Stats, StatsFilter},
  util::AppState,
};
use eframe::{
  egui::{
    containers, menu, style::Margin, CentralPanel, Context, CursorIcon, Event, Frame, Key,
    TopBottomPanel, Ui, Visuals,
  },
  emath::{self, Align2},
  epaint::Color32,
  glow,
};
use futures::executor::ThreadPoolBuilder;
use std::sync::{atomic::Ordering, Arc};

macro_rules! cmd {
  ($key:literal) => {
    if cfg!(macos) {
      concat!("⌘ + ", $key)
    } else {
      concat!("Ctrl + ", $key)
    }
  };
}

fn top_panel<R>(ctx: &Context, contents: impl FnOnce(&mut Ui) -> R) {
  const MENU: &str = "Menu";
  TopBottomPanel::top(MENU)
    .frame(
      Frame::none()
        .inner_margin(Margin::symmetric(8.0, 2.0))
        .fill(Color32::from_gray(40)),
    )
    .show(ctx, contents);
}

fn central_panel<R>(ctx: &Context, contents: impl FnOnce(&mut Ui) -> R) {
  CentralPanel::default()
    .frame(
      Frame::none()
        .inner_margin(Margin::same(8.0))
        .fill(Color32::from_gray(32)),
    )
    .show(ctx, contents);
}

fn bottom_panel<R>(page: Page, ctx: &Context, contents: impl FnOnce(&mut Ui) -> R) {
  let (id, margin) = match page {
    Page::Stats => ("stats_status", Margin::symmetric(8.0, 2.0)),
    // We need a little more vertical space for the chronometer status area so that it looks good.
    Page::Chronometer => ("chronometer_status", Margin::symmetric(8.0, 6.0)),
    // The experience page doesn't have a status area.
    Page::Experience => unreachable!(),
    Page::Offline => ("offline_status", Margin::symmetric(8.0, 2.0)),
  };

  TopBottomPanel::bottom(id)
    .frame(
      Frame::none()
        .inner_margin(margin)
        .fill(Color32::from_gray(40)),
    )
    .show(ctx, contents);
}

fn menu_item(ui: &mut Ui, close: bool, text: &str, hotkey: Option<&str>) -> bool {
  let response = ui.button(text);
  if response.clicked() || close {
    ui.close_menu();
  } else if let Some(hotkey) = hotkey {
    let cursor_pos = response.ctx.input().pointer.hover_pos();
    if let Some(pos) = cursor_pos {
      // Show the hotkey as a tooltip even if the menu item is disabled.
      if response.rect.contains(pos) && response.ctx.layer_id_at(pos) == Some(response.layer_id) {
        let pos = Some(pos + emath::vec2(16.0, 16.0));
        containers::show_tooltip_at(&response.ctx, response.id.with("_hotkey"), pos, |ui| {
          ui.label(hotkey);
        });
      }
    }
  }

  response.clicked()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Page {
  Stats,
  Chronometer,
  Experience,
  Offline,
}

pub struct App {
  // State.
  state: Arc<AppState>,
  page: Page,

  // Tab pages.
  stats: Stats,
  chronometer: Chronometer,
  experience: Experience,
  offline: Offline,

  // Dialogs.
  about_dlg: AboutDlg,
  confirm_dlg: ConfirmDlg,
  file_dlg: Option<egui_file::FileDialog>,
}

impl App {
  pub const fn inner_window_size() -> emath::Vec2 {
    emath::vec2(480.0, 640.0)
  }

  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    cc.egui_ctx.set_visuals(Visuals::dark());
    let mut style = (*cc.egui_ctx.style()).clone();

    // Make the "extreme" background color somewhat less extreme.
    style.visuals.extreme_bg_color = Color32::from_gray(20);

    // Make the fonts a bit bigger.
    for id in style.text_styles.values_mut() {
      id.size *= 1.1;
    }

    cc.egui_ctx.set_style(style);

    // Threading.
    let cpus = std::cmp::max(1, num_cpus::get()) + 1;
    let thread_pool = ThreadPoolBuilder::new().pool_size(cpus).create().unwrap();
    let thread_pool = Arc::new(thread_pool);

    // State.
    let state = Arc::new(AppState::default());
    let page = Page::Stats;

    // Tab pages.
    let ctx = &cc.egui_ctx;
    let log_path = config::get_log_path(cc.storage.unwrap()).unwrap_or_default();
    let stats = Stats::new(ctx, log_path, state.clone(), thread_pool.clone());
    let chronometer = Chronometer::new(thread_pool);
    let experience = Experience::new();
    let offline = Offline::new();

    // Dialog windows.
    let about_dlg = AboutDlg::new(state.clone());
    let confirm_dlg = ConfirmDlg::new(state.clone());
    let file_dlg = None;

    App {
      state,
      page,
      stats,
      chronometer,
      experience,
      offline,
      about_dlg,
      confirm_dlg,
      file_dlg,
    }
  }

  fn handle_hotkeys(&mut self, ctx: &Context, frame: &mut eframe::Frame) -> bool {
    let mut handled = false;
    for event in &ctx.input().events {
      if let Event::Key {
        key,
        pressed,
        modifiers,
      } = event
      {
        if *pressed && self.state.enabled.load(Ordering::Relaxed) {
          match key {
            Key::Escape if self.page == Page::Stats && !self.stats.filter().is_none() => {
              self.stats.set_filter(StatsFilter::None);
              handled = true;
            }
            Key::F
              if modifiers.command_only()
                && self.page == Page::Stats
                && !self.stats.stats().is_empty() =>
            {
              self.stats.show_filter_dlg();
              handled = true;
            }
            Key::L
              if modifiers.command_only()
                && self.page == Page::Stats
                && !self.stats.avatar().is_empty() =>
            {
              self.stats.show_search_dlg();
              handled = true;
            }
            Key::Q if modifiers.command_only() => {
              frame.close();
              handled = true;
            }
            Key::R
              if modifiers.command_only()
                && self.page == Page::Stats
                && !self.stats.stats().is_empty()
                && !self.stats.filter().is_resists() =>
            {
              self.stats.set_filter(StatsFilter::Resists);
              handled = true;
            }
            Key::R if modifiers.command_only() && self.offline.is_modified() => {
              self.offline.store();
              handled = true;
            }
            Key::F5 if self.page == Page::Stats => {
              self.stats.reload(ctx);
              handled = true;
            }
            _ => (),
          }
        }
      }
    }
    handled
  }

  fn choose_folder_path(&mut self, ctx: &Context) {
    let path = Some(self.stats.log_path().into());
    let mut file_dlg = egui_file::FileDialog::select_folder(path)
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .current_pos([0.0, 24.0])
      .default_size(ctx.available_rect().size())
      .filter("txt".into())
      .show_new_folder(false)
      .show_rename(false)
      .resizable(false);
    file_dlg.open();

    self.state.enabled.store(false, Ordering::Relaxed);
    self.file_dlg = Some(file_dlg);
  }

  fn choose_load_path(&mut self, ctx: &Context) {
    if self.offline.is_modified() {
      // Current save-game is modified, deal with that first.
      if let Some(file_name) = self.offline.file_name() {
        self.confirm_dlg.open(file_name, Hence::Load);
        return;
      }
    }

    let path = some!(config::get_sota_config_path());
    let path = path.join("SavedGames");
    let mut file_dlg = egui_file::FileDialog::open_file(Some(path))
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .current_pos([0.0, 24.0])
      .default_size(ctx.available_rect().size())
      .filter("sota".into())
      .show_new_folder(false)
      .resizable(false);
    file_dlg.open();

    self.state.enabled.store(false, Ordering::Relaxed);
    self.file_dlg = Some(file_dlg);
  }

  fn choose_store_path(&mut self, ctx: &Context) {
    let path = some!(self.offline.file_path());
    let mut file_dlg = egui_file::FileDialog::save_file(Some(path))
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .current_pos([0.0, 24.0])
      .default_size(ctx.available_rect().size())
      .filter("sota".into())
      .show_new_folder(false)
      .resizable(false);
    file_dlg.open();

    self.state.enabled.store(false, Ordering::Relaxed);
    self.file_dlg = Some(file_dlg);
  }
}

impl eframe::App for App {
  fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
    // Process load request from the offline page.
    if self.offline.load_request() {
      self.choose_load_path(ctx);
    }

    // Set the progress cursor if the app is busy.
    if self.state.busy.load(Ordering::Relaxed) {
      ctx.output().cursor_icon = CursorIcon::Progress;
    }

    // We want to close any open menu whenever a hotkey is processed.
    let close_menu = self.handle_hotkeys(ctx, frame);

    // Top panel for the menu bar.
    let enabled = self.state.enabled.load(Ordering::Relaxed);
    top_panel(ctx, |ui| {
      ui.set_enabled(enabled);
      ui.horizontal_centered(|ui| {
        menu::bar(ui, |ui| {
          ui.menu_button("File", |ui| {
            match self.page {
              Page::Stats => {
                if menu_item(ui, close_menu, "Set Log Folder...", None) {
                  self.choose_folder_path(ctx);
                }

                let enabled = !self.stats.avatar().is_empty();
                ui.add_enabled_ui(enabled, |ui| {
                  if menu_item(ui, close_menu, "Search Logs...", Some(cmd!("L"))) {
                    self.stats.show_search_dlg();
                  }
                });

                if menu_item(ui, close_menu, "Reload Stats", Some("F5")) {
                  self.stats.reload(ui.ctx());
                }

                ui.separator();
              }
              Page::Offline => {
                if menu_item(ui, close_menu, "Load Save-game...", None) {
                  self.choose_load_path(ctx);
                }

                let enabled = self.offline.is_modified();
                ui.add_enabled_ui(enabled, |ui| {
                  if menu_item(ui, close_menu, "Store Save-game...", Some(cmd!("S"))) {
                    self.offline.store();
                  }
                });

                let enabled = self.offline.is_loaded();
                ui.add_enabled_ui(enabled, |ui| {
                  if menu_item(ui, close_menu, "Store Save-game as...", None) {
                    self.choose_store_path(ctx);
                  }
                });

                ui.separator();
              }
              _ => (),
            }

            if menu_item(ui, close_menu, "Quit", Some(cmd!("Q"))) {
              frame.close();
            }
          });
          if self.page == Page::Stats {
            ui.menu_button("View", |ui| {
              let enabled = !self.stats.filter().is_resists() && !self.stats.stats().is_empty();
              ui.add_enabled_ui(enabled, |ui| {
                if menu_item(ui, close_menu, "Effective Resists", Some(cmd!("R"))) {
                  self.stats.set_filter(StatsFilter::Resists);
                }
              });

              let enabled = !self.stats.stats().is_empty();
              ui.add_enabled_ui(enabled, |ui| {
                if menu_item(ui, close_menu, "Filter Stats...", Some(cmd!("F"))) {
                  self.stats.show_filter_dlg();
                }
              });

              let enabled = !self.stats.filter().is_none();
              ui.add_enabled_ui(enabled, |ui| {
                if menu_item(ui, close_menu, "Reset View", Some("Esc")) {
                  self.stats.set_filter(StatsFilter::None);
                }
              });
            });
          }
          ui.menu_button("Help", |ui| {
            if menu_item(ui, close_menu, "About...", None) {
              self.about_dlg.open();
            }
          });
        });
      });
    });

    // Put the dialogs here so that they're anchored below the menu-bar.
    if let Some(file_dlg) = &mut self.file_dlg {
      if !file_dlg.show(ctx).visible() {
        if file_dlg.selected() {
          if let Some(path) = file_dlg.path() {
            match file_dlg.dialog_type() {
              egui_file::DialogType::SelectFolder => {
                config::set_log_path(frame.storage_mut().unwrap(), &path);
                self.stats.set_log_path(ctx, path);
              }
              egui_file::DialogType::OpenFile => self.offline.load(path),
              egui_file::DialogType::SaveFile => self.offline.store_as(path),
            }
          }
        }
        self.state.enabled.store(true, Ordering::Relaxed);
        self.file_dlg = None;
      }
    }

    if !self.confirm_dlg.show(ctx) {
      match self.confirm_dlg.take_choice() {
        Some(Choice::Save) => self.offline.store(),
        Some(Choice::Discard) => self.offline.discard(),
        _ => (),
      }
      match self.confirm_dlg.take_hence() {
        Some(Hence::Load) => self.choose_load_path(ctx),
        Some(Hence::Exit) => frame.close(),
        None => (),
      }
    }

    self.about_dlg.show(ctx);

    // Bottom panel for the status. This needs to be done before
    // the central panel so that we know how much space is left.
    match self.page {
      Page::Stats => bottom_panel(Page::Stats, ctx, |ui| {
        ui.set_enabled(enabled);
        self.stats.show_status(ui);
      }),
      Page::Chronometer => bottom_panel(Page::Chronometer, ctx, |ui| {
        ui.set_enabled(enabled);
        self.chronometer.show_status(ui);
      }),
      Page::Offline => bottom_panel(Page::Offline, ctx, |ui| {
        ui.set_enabled(enabled);
        self.offline.show_status(ui);
      }),
      _ => (),
    }

    // Central panel for the tab pages.
    central_panel(ctx, |ui| {
      ui.set_enabled(enabled);
      ui.horizontal(|ui| {
        let button = ui.selectable_value(&mut self.page, Page::Stats, "Stats");
        if button.clicked() {
          self.chronometer.stop_timer();
        }

        let button = ui.selectable_value(&mut self.page, Page::Chronometer, "Chronometer");
        if button.clicked() {
          self.chronometer.start_timer(ctx);
        }

        let button = ui.selectable_value(&mut self.page, Page::Experience, "Experience");
        if button.clicked() {
          self.chronometer.stop_timer();
        }

        let button = ui.selectable_value(&mut self.page, Page::Offline, "Offline");
        if button.clicked() {
          self.chronometer.stop_timer();
        }
      });
      ui.separator();
      match self.page {
        Page::Stats => self.stats.show(ui, frame),
        Page::Chronometer => self.chronometer.show(ui),
        Page::Experience => self.experience.show(ui),
        Page::Offline => self.offline.show(ui),
      }
    });
  }

  fn persist_egui_memory(&self) -> bool {
    false
  }

  fn on_close_event(&mut self) -> bool {
    if !self.offline.is_modified() {
      return true;
    }

    if !self.confirm_dlg.visible() {
      if let Some(file_name) = self.offline.file_name() {
        self.confirm_dlg.open(file_name, Hence::Exit);
      }
    }

    false
  }

  fn on_exit(&mut self, _: Option<&glow::Context>) {
    self.stats.on_exit();
    self.chronometer.stop_timer();
  }
}
