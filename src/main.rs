// Don't show the console on Windows.
#![windows_subsystem = "windows"]

#[macro_use]
mod util;

mod about_dlg;
mod app;
mod chronometer;
mod config;
mod confirm_dlg;
mod experience;
mod game_data;
mod items_dlg;
mod log_data;
mod log_dlg;
mod notes_dlg;
mod offline;
mod search_dlg;
mod stats;

use app::App;
use util::{APP_ICON, APP_TITLE};

fn main() {
  let icon = image::load_from_memory(APP_ICON).unwrap();
  let options = eframe::NativeOptions {
    resizable: false,
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

  eframe::run_native(APP_TITLE, options, Box::new(|cc| Box::new(App::new(cc))));
}
