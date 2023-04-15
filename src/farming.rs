use crate::{
  config,
  plant_dlg::PlantDlg,
  plant_info::{Event, Plant},
  util::{AppState, Cancel, FAIL_ERR, NONE_ERR},
};
use eframe::{
  egui::{Context, Label, ScrollArea, Ui, WidgetText},
  epaint::Color32,
  Storage,
};
use notify_rust::Notification;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
  thread::{self, JoinHandle},
  time::{Duration, Instant},
};

pub struct Farming {
  plant_dlg: PlantDlg,
  plants: Arc<Mutex<Vec<Plant>>>,
  persist: Arc<AtomicBool>,
  cancel: Option<Cancel>,
  thread: Option<JoinHandle<()>>,
}

impl Farming {
  pub fn new(ctx: Context, storage: &dyn Storage, state: AppState) -> Self {
    let plant_dlg = PlantDlg::new(state);
    let plants = config::get_plants(storage).unwrap_or_default();
    let plants = Arc::new(Mutex::new(plants));
    let persist = Arc::new(AtomicBool::new(false));
    let cancel = Cancel::default();
    Self {
      plant_dlg,
      plants: plants.clone(),
      persist: persist.clone(),
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
              err!(Notification::new().summary(summary).body(&body).show());
            }

            // Flag that the timers need to be persisted.
            persist.store(true, Ordering::Relaxed);

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
    if !self.plant_dlg.show(ui.ctx()) {
      if let Some(plant_info) = self.plant_dlg.take_result() {
        self.plants.lock().expect(FAIL_ERR).push(plant_info);
        self.persist.store(true, Ordering::Relaxed);
      }
    }

    // Tool bar.
    ui.horizontal(|ui| {
      if ui.button("Add Timer").clicked() {
        self.plant_dlg.open();
      }
    });

    ui.separator();

    // Timer list.
    ScrollArea::vertical()
      .id_source("farming_scroll_area")
      .show(ui, |ui| {
        let mut lock = self.plants.lock().expect(FAIL_ERR);
        let mut index = 0;
        while index < lock.len() {
          let mut delete = false;
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

              // Planting environment.
              let environment = plant.environment();
              let date_time = plant.date_time().format("%Y-%m-%d %H:%M");
              ui.separator();
              ui.label(format!("{environment:?} {date_time}",));

              // Next event.
              let (event, date_time) = plant.next_event();
              if event != Event::None {
                let date_time = date_time.format("%Y-%m-%d %H:%M");
                ui.separator();
                ui.label(format!("{event:?} {date_time}"));
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
                    self.persist.store(true, Ordering::Relaxed);
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

            col[0].visuals_mut().widgets.noninteractive.bg_stroke.color = Color32::from_gray(45);
            col[0].separator();
          });

          if delete {
            lock.remove(index);
            self.persist.store(true, Ordering::Relaxed);
          } else {
            index += 1;
          }
        }

        if self.persist.swap(false, Ordering::Relaxed) {
          // Persist the timers.
          config::set_plants(frame.storage_mut().expect(NONE_ERR), &lock);
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
      thread.join().expect(FAIL_ERR);
    }
  }
}
