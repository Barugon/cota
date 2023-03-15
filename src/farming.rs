use eframe::egui::Ui;

pub struct Farming {}

impl Farming {
  pub fn new() -> Self {
    Self {}
  }

  pub fn show(&mut self, ui: &mut Ui) {
    // Tool bar.
    ui.horizontal(|ui| if ui.button("Add Timer").clicked() {});
  }
}
