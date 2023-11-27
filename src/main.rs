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
mod storage;
mod towns_dlg;

use app::App;
use config::Config;
use eframe::{
  egui::{IconData, ViewportBuilder},
  AppCreator,
};
use util::{APP_ICON, APP_TITLE};

fn main() {
  let config = Config::new().unwrap();
  let icon = image::load_from_memory(APP_ICON).unwrap();
  let icon = IconData {
    width: icon.width(),
    height: icon.height(),
    rgba: icon.into_rgba8().into_raw(),
  };
  let viewport = ViewportBuilder::default()
    .with_resizable(false)
    .with_inner_size(App::inner_window_size())
    .with_max_inner_size(App::inner_window_size())
    .with_min_inner_size(App::inner_window_size())
    .with_icon(icon);

  let options = eframe::NativeOptions {
    viewport,
    ..Default::default()
  };

  let creator: AppCreator = Box::new(move |cc| Box::new(App::new(cc, config)));
  eframe::run_native(APP_TITLE, options, creator).unwrap();
}
