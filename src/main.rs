// Don't show the console on Windows.
#![windows_subsystem = "windows"]

#[macro_use]
mod util;

mod about_dlg;
mod app;
mod chronometer;
mod config;
mod confirm_dlg;
mod dps_dlg;
mod ethos;
mod experience;
mod farming;
mod game_data;
mod items_dlg;
mod log_data;
mod log_dlg;
mod notes_dlg;
mod offline;
mod plant_dlg;
mod plant_info;
mod search_dlg;
mod skill_info;
mod stats;
mod towns_dlg;

use app::App;
use config::Config;
use eframe::AppCreator;
use util::{APP_ICON, APP_TITLE, FAIL_ERR, NONE_ERR};

fn main() {
  let config = Config::new().expect(NONE_ERR);
  let icon = image::load_from_memory(APP_ICON).expect(FAIL_ERR);
  let options = eframe::NativeOptions {
    resizable: false,
    initial_window_pos: config.get_window_pos(),
    initial_window_size: Some(App::inner_window_size()),
    max_window_size: Some(App::inner_window_size()),
    min_window_size: Some(App::inner_window_size()),
    icon_data: Some(eframe::IconData {
      width: icon.width(),
      height: icon.height(),
      rgba: icon.into_rgba8().into_raw(),
    }),
    ..Default::default()
  };

  let creator: AppCreator = Box::new(move |cc| Box::new(App::new(cc, config)));
  eframe::run_native(APP_TITLE, options, creator).expect(FAIL_ERR);
}
