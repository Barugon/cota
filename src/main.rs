// Don't show the console on Windows.
#![windows_subsystem = "windows"]

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
mod storage;
mod towns_dlg;
mod util;

use app::App;
use config::Config;
use eframe::{
  AppCreator,
  egui::{IconData, ViewportBuilder},
};
use util::{APP_ICON, APP_NAME, APP_TITLE};

fn main() {
  let config = Config::new().unwrap();
  let icon = image::load_from_memory(APP_ICON).unwrap();
  let icon = IconData {
    width: icon.width(),
    height: icon.height(),
    rgba: icon.into_rgba8().into_raw(),
  };

  let viewport = ViewportBuilder::default()
    .with_app_id(APP_NAME)
    .with_icon(icon)
    .with_inner_size(App::inner_window_size())
    .with_max_inner_size(App::inner_window_size())
    .with_maximize_button(false)
    .with_min_inner_size(App::inner_window_size())
    .with_resizable(false)
    .with_title(APP_TITLE);

  let options = eframe::NativeOptions {
    viewport,
    ..Default::default()
  };

  let creator: AppCreator = Box::new(move |cc| Ok(Box::new(App::new(cc, config))));
  eframe::run_native(APP_NAME, options, creator).unwrap();
}
