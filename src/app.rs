use crate::{
  about_dlg::AboutDlg,
  chronometer::Chronometer,
  config::Config,
  confirm_dlg::{Choice, ConfirmDlg, Hence},
  experience::Experience,
  farming::Farming,
  offline::Offline,
  stats::{Stats, StatsFilter},
  util::{self, AppState, Page},
};
use eframe::{
  egui::{
    Button, CentralPanel, Context, CursorIcon, Event, Frame, Key, Margin, TextWrapMode, TopBottomPanel, Ui,
    ViewportCommand, Visuals, menu,
  },
  emath::Align2,
  epaint::{self, Color32, Vec2},
  glow,
};
use futures::executor::ThreadPoolBuilder;
use std::{ffi::OsStr, path::Path};

#[cfg(target_os = "macos")]
macro_rules! cmd {
  ($key:literal) => {
    concat!("⌘ + ", $key)
  };
}

#[cfg(not(target_os = "macos"))]
macro_rules! cmd {
  ($key:literal) => {
    concat!("Ctrl + ", $key)
  };
}

pub struct App {
  // State.
  config: Config,
  state: AppState,
  page: Page,

  // Tab pages.
  chronometer: Chronometer,
  experience: Experience,
  farming: Farming,
  offline: Offline,
  stats: Stats,

  // Dialogs.
  about_dlg: AboutDlg,
  confirm_dlg: ConfirmDlg,
  file_dlg: Option<egui_file::FileDialog>,
}

impl App {
  pub const fn inner_window_size() -> Vec2 {
    epaint::vec2(480.0, 640.0)
  }

  pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
    egui_extras::install_image_loaders(&cc.egui_ctx);

    cc.egui_ctx.set_visuals(Visuals::dark());
    let mut style = (*cc.egui_ctx.style()).clone();

    // Make the "extreme" background color somewhat less extreme.
    style.visuals.extreme_bg_color = Color32::from_gray(20);

    // Make the fonts a bit bigger.
    for id in style.text_styles.values_mut() {
      id.size *= 1.1;
    }

    cc.egui_ctx.all_styles_mut(|s| *s = style.clone());

    // Threading.
    let count = std::cmp::max(2, num_cpus::get());
    let threads = ThreadPoolBuilder::new().pool_size(count).create().unwrap();

    // State.
    let locale = util::get_locale();
    let state = AppState::default();
    let page = config.get_page().unwrap_or(Page::Chronometer);

    // Tab pages.
    let log_path = config.get_log_path().unwrap_or_default();
    let mut chronometer = Chronometer::new(threads.clone(), state.clone());
    let experience = Experience::new(log_path.clone(), threads.clone(), config.clone(), state.clone(), locale);
    let farming = Farming::new(cc.egui_ctx.clone(), config.clone(), state.clone());
    let offline = Offline::new(state.clone());
    let stats = Stats::new(log_path, threads, config.clone(), state.clone(), locale);

    if page == Page::Chronometer {
      // Start the chronometer timer.
      chronometer.start_timer(cc.egui_ctx.clone());
    }

    // Dialog windows.
    let about_dlg = AboutDlg::new(state.clone());
    let confirm_dlg = ConfirmDlg::new(state.clone());
    let file_dlg = None;

    App {
      config,
      state,
      page,
      chronometer,
      experience,
      farming,
      offline,
      stats,
      about_dlg,
      confirm_dlg,
      file_dlg,
    }
  }

  fn handle_input(&mut self, ctx: &Context) -> bool {
    let mut close_status = CloseStatus::None;
    let mut handled = false;
    ctx.input(|state| {
      if state.viewport().close_requested() {
        if self.offline.changed() {
          self.offline.on_close_event();
          if !self.confirm_dlg.visible() {
            let file_name = self.offline.file_name().unwrap();
            self.confirm_dlg.open(file_name, Hence::Exit);
          }
          close_status = CloseStatus::CancelClose;
        }
        return;
      }

      for event in &state.events {
        if let Event::Key {
          key,
          physical_key: _,
          pressed,
          repeat,
          modifiers,
        } = event
        {
          if *pressed && !*repeat && !self.state.is_disabled() {
            match key {
              Key::Escape if self.page == Page::Stats && !self.stats.filter().is_none() => {
                self.stats.set_filter(StatsFilter::None);
                handled = true;
              }
              Key::D if modifiers.command_only() && self.page == Page::Stats && !self.stats.avatar().is_empty() => {
                self.stats.show_dps_dlg();
                handled = true;
              }
              Key::F if modifiers.command_only() && self.page == Page::Stats && !self.stats.stats().is_empty() => {
                self.stats.show_filter_dlg();
                handled = true;
              }
              Key::L if modifiers.command_only() && self.page == Page::Stats && !self.stats.avatar().is_empty() => {
                self.stats.show_search_dlg();
                handled = true;
              }
              Key::Q if modifiers.command_only() => {
                close_status = CloseStatus::Close;
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
              Key::S if modifiers.command_only() && self.offline.changed() => {
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
    });

    match close_status {
      CloseStatus::None => (),
      CloseStatus::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
      CloseStatus::CancelClose => ctx.send_viewport_cmd(ViewportCommand::CancelClose),
    }

    handled
  }

  fn choose_folder_path(&mut self, ctx: &Context) {
    let path = Some(self.stats.log_path().into());
    let filter = Box::new({
      let ext = Some(OsStr::new("txt"));
      move |path: &Path| {
        const PREFIX: &[u8] = "SotAChatLog_".as_bytes();
        let Some(name) = path.file_name() else {
          return false;
        };
        let name = name.as_encoded_bytes();
        name.starts_with(PREFIX) && path.extension() == ext
      }
    });

    let available = ctx.available_rect().size();
    let mut file_dlg = egui_file::FileDialog::select_folder(path)
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .current_pos([0.0, 24.0])
      .default_size([available.x, available.y * 0.5])
      .show_files_filter(filter)
      .show_new_folder(false)
      .show_rename(false)
      .resizable(false);
    file_dlg.open();

    self.state.set_disabled(true);
    self.file_dlg = Some(file_dlg);
  }

  fn choose_load_path(&mut self, ctx: &Context) {
    if self.offline.changed() {
      // Current save-game is modified, deal with that first.
      if let Some(file_name) = self.offline.file_name() {
        self.confirm_dlg.open(file_name, Hence::Load);
        return;
      }
    }

    let Some(path) = self.config.get_save_game_path() else {
      return;
    };

    let filter = Box::new({
      let ext = Some(OsStr::new(App::SOTA));
      move |path: &Path| path.extension() == ext
    });

    let available = ctx.available_rect().size();
    let mut file_dlg = egui_file::FileDialog::open_file(Some(path))
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .current_pos([0.0, 24.0])
      .default_size([available.x, available.y * 0.5])
      .show_files_filter(filter)
      .show_new_folder(false)
      .resizable(false);
    file_dlg.open();

    self.state.set_disabled(true);
    self.file_dlg = Some(file_dlg);
  }

  fn choose_store_path(&mut self, ctx: &Context) {
    let Some(path) = self.offline.file_path() else {
      return;
    };

    let filter = Box::new({
      let ext = Some(OsStr::new(App::SOTA));
      move |path: &Path| path.extension() == ext
    });

    let available = ctx.available_rect().size();
    let mut file_dlg = egui_file::FileDialog::save_file(Some(path))
      .anchor(Align2::CENTER_TOP, [0.0, 0.0])
      .current_pos([0.0, 24.0])
      .default_size([available.x, available.y * 0.5])
      .show_files_filter(filter)
      .show_new_folder(false)
      .resizable(false);
    file_dlg.open();

    self.state.set_disabled(true);
    self.file_dlg = Some(file_dlg);
  }

  const SOTA: &str = "sota";
}

impl eframe::App for App {
  fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
    // Process load request from the offline page.
    if self.offline.load_request() {
      self.choose_load_path(ctx);
    }

    // Set the progress cursor if the app is busy.
    if self.state.is_busy() {
      ctx.output_mut(|output| output.cursor_icon = CursorIcon::Progress);
    }

    // We want to close any open menu whenever a hotkey is processed.
    let close_menu = self.handle_input(ctx);

    // Top panel for the menu bar.
    let enabled = !self.state.is_disabled();
    top_panel(ctx, |ui| {
      if !enabled {
        ui.disable();
      }
      ui.horizontal_centered(|ui| {
        menu::bar(ui, |ui| {
          ui.menu_button("File", |ui| {
            if menu_item(ui, close_menu, "Set Log Folder...", None) {
              self.choose_folder_path(ctx);
            }

            match self.page {
              Page::Offline => {
                ui.separator();

                if menu_item(ui, close_menu, "Load Save-game...", None) {
                  self.choose_load_path(ctx);
                }

                let enabled = self.offline.changed();
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
              }
              Page::Stats => {
                ui.separator();

                let enabled = !self.stats.avatar().is_empty();
                ui.add_enabled_ui(enabled, |ui| {
                  if menu_item(ui, close_menu, "Search Logs...", Some(cmd!("L"))) {
                    self.stats.show_search_dlg();
                  }
                });

                ui.add_enabled_ui(enabled, |ui| {
                  if menu_item(ui, close_menu, "Tally DPS...", Some(cmd!("D"))) {
                    self.stats.show_dps_dlg();
                  }
                });

                if menu_item(ui, close_menu, "Reload Stats", Some("F5")) {
                  self.stats.reload(ui.ctx());
                }
              }
              _ => (),
            }

            ui.separator();

            if menu_item(ui, close_menu, "Quit", Some(cmd!("Q"))) {
              ctx.send_viewport_cmd(ViewportCommand::Close);
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
                self.config.set_log_path(path);
                self.experience.set_log_path(ctx, path.to_owned());
                self.stats.set_log_path(ctx, path.to_owned());
              }
              egui_file::DialogType::OpenFile => {
                let folder = path.with_file_name(String::default());
                if self.offline.load(path.to_owned()) {
                  self.config.set_save_game_path(&folder);
                }
              }
              egui_file::DialogType::SaveFile => self.offline.store_as(path.to_owned()),
            }
          }
        }
        self.state.set_disabled(false);
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
        Some(Hence::Exit) => ctx.send_viewport_cmd(ViewportCommand::Close),
        None => (),
      }
    }

    self.about_dlg.show(ctx);

    // Bottom panel for the status. This needs to be done before
    // the central panel so that we know how much space is left.
    match self.page {
      Page::Chronometer => bottom_panel(Page::Chronometer, ctx, |ui| {
        if !enabled {
          ui.disable();
        }
        self.chronometer.show_status(ui);
      }),
      Page::Offline => bottom_panel(Page::Offline, ctx, |ui| {
        if !enabled {
          ui.disable();
        }
        self.offline.show_status(ui);
      }),
      Page::Stats => bottom_panel(Page::Stats, ctx, |ui| {
        if !enabled {
          ui.disable();
        }
        self.stats.show_status(ui);
      }),
      _ => (),
    }

    // Central panel for the tab pages.
    central_panel(ctx, |ui| {
      if !enabled {
        ui.disable();
      }

      // Tab control.
      ui.horizontal(|ui| {
        let button = ui.selectable_value(&mut self.page, Page::Chronometer, "Chronometer");
        if button.clicked() {
          self.chronometer.start_timer(ctx.clone());
          self.config.set_page(Page::Chronometer);
        }

        let button = ui.selectable_value(&mut self.page, Page::Experience, "Experience");
        if button.clicked() {
          self.chronometer.stop_timer();
          self.config.set_page(Page::Experience);
        }

        let button = ui.selectable_value(&mut self.page, Page::Farming, "Farming");
        if button.clicked() {
          self.chronometer.stop_timer();
          self.config.set_page(Page::Farming);
        }

        let button = ui.selectable_value(&mut self.page, Page::Offline, "Offline");
        if button.clicked() {
          self.chronometer.stop_timer();
          self.config.set_page(Page::Offline);
        }

        let button = ui.selectable_value(&mut self.page, Page::Stats, "Stats");
        if button.clicked() {
          self.chronometer.stop_timer();
          self.config.set_page(Page::Stats);
        }
      });

      ui.separator();

      // Tab pages.
      match self.page {
        Page::Chronometer => self.chronometer.show(ui),
        Page::Experience => self.experience.show(ui),
        Page::Farming => self.farming.show(ui),
        Page::Offline => self.offline.show(ui),
        Page::Stats => self.stats.show(ui),
      }
    });
  }

  fn on_exit(&mut self, _: Option<&glow::Context>) {
    self.chronometer.on_exit();
    self.experience.on_exit();
    self.farming.on_exit();
    self.stats.on_exit();
  }
}

enum CloseStatus {
  None,
  Close,
  CancelClose,
}

fn top_panel<R>(ctx: &Context, contents: impl FnOnce(&mut Ui) -> R) {
  const MENU: &str = "Menu";
  TopBottomPanel::top(MENU)
    .frame(
      Frame::NONE
        .inner_margin(Margin::symmetric(8, 2))
        .fill(Color32::from_gray(40)),
    )
    .show(ctx, contents);
}

fn central_panel<R>(ctx: &Context, contents: impl FnOnce(&mut Ui) -> R) {
  CentralPanel::default()
    .frame(Frame::NONE.inner_margin(Margin::same(8)).fill(Color32::from_gray(32)))
    .show(ctx, contents);
}

fn bottom_panel<R>(page: Page, ctx: &Context, contents: impl FnOnce(&mut Ui) -> R) {
  let (id, margin) = match page {
    // We need a little more vertical space for the chronometer status area so that it looks good.
    Page::Chronometer => ("chronometer_status", Margin::symmetric(8, 6)),
    // The experience page doesn't have a status area.
    Page::Experience => unreachable!(),
    // The farming page doesn't have a status area.
    Page::Farming => unreachable!(),
    Page::Offline => ("offline_status", Margin::symmetric(8, 4)),
    Page::Stats => ("stats_status", Margin::symmetric(8, 4)),
  };

  TopBottomPanel::bottom(id)
    .frame(Frame::NONE.inner_margin(margin).fill(Color32::from_gray(40)))
    .show(ctx, contents);
}

fn menu_item(ui: &mut Ui, close: bool, text: &str, hotkey: Option<&str>) -> bool {
  let widget = if let Some(hotkey) = hotkey {
    Button::new(text).wrap_mode(TextWrapMode::Extend).shortcut_text(hotkey)
  } else {
    Button::new(text).wrap_mode(TextWrapMode::Extend)
  };

  let response = ui.add(widget);
  let clicked = response.clicked();
  if clicked || close {
    ui.close_menu();
  }

  clicked
}
