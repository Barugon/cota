use crate::{
  config,
  plant_dlg::PlantDlg,
  plant_info::{Event, PlantInfo},
  util::{AppState, Cancel, FAIL_ERR, NONE_ERR},
};
use eframe::{
  egui::{Context, Label, ScrollArea, Ui, WidgetText},
  epaint::Color32,
  Storage,
};
use notify_rust::Notification;
use std::{
  sync::{Arc, Mutex},
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

pub struct Farming {
  plant_dlg: PlantDlg,
  plants: Arc<Mutex<Vec<PlantInfo>>>,
  cancel: Option<Cancel>,
  thread: Option<JoinHandle<()>>,
}

impl Farming {
  pub fn new(ctx: Context, storage: &dyn Storage, state: AppState) -> Self {
    let plant_dlg = PlantDlg::new(state);
    let plants = config::get_plants(storage).unwrap_or_default();
    let plants = Arc::new(Mutex::new(plants));
    let cancel = Cancel::default();
    Self {
      plant_dlg,
      plants: plants.clone(),
      cancel: Some(cancel.clone()),
      thread: Some(thread::spawn(move || loop {
        let mut lock = plants.lock().expect(FAIL_ERR);
        for plant in lock.iter_mut() {
          if plant.check() {
            // Popup a desktop notification.
            let summary = match plant.current_event() {
              Event::None => Default::default(),
              Event::Water => "Water Plants",
              Event::Harvest => "Harvest Plants",
            };
            if !summary.is_empty() {
              let name = plant.seed_name();
              let env = plant.environment();
              let desc = plant.description();
              let body = if desc.is_empty() {
                format!("{name} | {env:?}")
              } else {
                format!("{name} | {env:?} | {desc}")
              };
              let _ = Notification::new().summary(summary).body(&body).show();
            }

            // Repaint.
            ctx.request_repaint();
          }
        }

        // Unlock the mutex.
        drop(lock);

        // Wait for five seconds.
        const DURATION: Duration = Duration::from_secs(5);
        let instant = Instant::now();
        while instant.elapsed() < DURATION {
          if cancel.is_canceled() {
            return;
          }

          // We need to sleep for some actual amount of time or this thread will peg one of the cores.
          thread::sleep(Duration::from_millis(10));
        }
      })),
    }
  }

  pub fn show(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
    // Tool bar.
    ui.horizontal(|ui| {
      if ui.button("Add Timer").clicked() {
        self.plant_dlg.open();
      }
    });

    if !self.plant_dlg.show(ui.ctx()) {
      if let Some(plant_info) = self.plant_dlg.take_result() {
        let mut lock = self.plants.lock().expect(FAIL_ERR);
        lock.push(plant_info);

        // Persist the timers.
        config::set_plants(frame.storage_mut().expect(NONE_ERR), &lock);
      }
    }

    ui.separator();

    ScrollArea::vertical()
      .id_source("farming_scroll_area")
      .show(ui, |ui| {
        let mut lock = self.plants.lock().expect(FAIL_ERR);
        let mut index = 0;
        while index < lock.len() {
          let mut delete = false;
          let mut store = false;
          let plant = &mut lock[index];
          let event = plant.current_event();
          let item_spacing = ui.spacing().item_spacing;

          // Use a single column in order to force the scroll area to fill the entire available width.
          ui.columns(1, |col| {
            col[0].horizontal(|ui| {
              // Seed name.
              let text = WidgetText::from(plant.seed_name());
              let text = match event {
                Event::None => text.color(Color32::from_rgb(220, 220, 220)),
                Event::Water => text.color(Color32::from_rgb(255, 255, 0)),
                Event::Harvest => text.color(Color32::from_rgb(0, 255, 0)),
              };
              ui.spacing_mut().item_spacing.x = item_spacing.x * 0.5;
              ui.label(text);
              ui.separator();

              // Planting environment.
              let text = plant.date_time().format("%Y-%m-%d %H:%M").to_string();
              let text = format!("{:?} {text}", plant.environment());
              ui.label(text);
              ui.separator();

              // Next event.
              let (event, date_time) = plant.next_event();
              if event != Event::None {
                let format = date_time.format("%Y-%m-%d %H:%M");
                let text = format!("{event:?} {}", format.to_string());
                let widget = Label::new(text).wrap(true);
                ui.add(widget);
              }
            });
            col[0].horizontal(|ui| {
              match event {
                Event::None => {
                  if ui.button("Discard").clicked() {
                    delete = true;
                  }
                }
                Event::Water => {
                  if ui.button("Water").clicked() {
                    plant.reset_events();
                    store = true;
                  }
                }
                Event::Harvest => {
                  if ui.button("Harvest").clicked() {
                    delete = true;
                  }
                }
              }

              let widget = Label::new(plant.description()).wrap(true);
              ui.spacing_mut().item_spacing.x = item_spacing.x;
              ui.add(widget);
            });
            col[0].separator();
          });

          if delete {
            lock.remove(index);
            config::set_plants(frame.storage_mut().expect(NONE_ERR), &lock);
          } else {
            index += 1;
            if store {
              config::set_plants(frame.storage_mut().expect(NONE_ERR), &lock);
            }
          }
        }
      });
  }

  pub fn stop_timer(&mut self) {
    // Cancel the thread.
    if let Some(mut cancel) = self.cancel.take() {
      cancel.cancel();
    }

    // Wait for it to join.
    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
  }
}
