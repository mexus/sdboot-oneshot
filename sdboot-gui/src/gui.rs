use std::sync::Arc;

use egui::TextStyle;

use crate::Manager;

/// GUI application.
pub struct GuiApplication {
    manager: Manager,
    entries: Arc<[Arc<str>]>,
    selected: Option<Arc<str>>,
    message: String,
}

impl Default for GuiApplication {
    fn default() -> Self {
        let manager = Manager::new();
        let entries: Vec<Arc<str>> = manager
            .entries()
            .expect("Unable to load entries")
            .into_iter()
            .map(Arc::from)
            .collect();
        let selected = manager
            .get_oneshot()
            .expect("Unable to load current entry")
            .map(Arc::from);
        Self {
            manager,
            entries: Arc::from(entries),
            selected,
            message: String::new(),
        }
    }
}

impl eframe::App for GuiApplication {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Boot entries");

            for entry in self.entries.iter() {
                ui.radio_value(&mut self.selected, Some(Arc::clone(entry)), entry as &str);
            }

            if ui.button("Unset").clicked() {
                log::info!("Removing oneshot entry");
                self.selected = None;
                if let Err(e) = self.manager.remove_oneshot() {
                    log::error!("Unable to remove oneshot entry: {:#}", e);
                    self.message = format!("Unable to remove oneshot entry: {:#}", e);
                } else {
                    self.message = "Oneshot entry unset".to_string();
                    self.selected = None;
                }
            }

            if ui.button("Apply").clicked() {
                if let Some(selected) = &self.selected {
                    log::info!("Setting oneshot entry to {}", selected);
                    if let Err(e) = self.manager.set_oneshot(selected) {
                        log::error!("Unable to set oneshot entry to {}: {:#}", selected, e);
                        self.message =
                            format!("Unable to set oneshot entry to {}: {:#}", selected, e);
                    } else {
                        self.message = format!("Oneshot entry set to {}", selected);
                    }
                } else {
                    self.message = "No entry selected!".to_string();
                }
            }

            if !self.message.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    let spacing =
                        ui.fonts(|f| f.glyph_width(&TextStyle::Body.resolve(ui.style()), ' '));
                    ui.spacing_mut().item_spacing.x = spacing;
                    ui.label(&self.message);
                    if ui.button("📋").clicked() {
                        if let Err(e) = copy_to_clipboard(&self.message) {
                            log::error!("Unable to copy message to the clipboard: {e:#}")
                        }
                    }
                });
            }
        });
    }
}

fn copy_to_clipboard(value: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    let mut clipboard = arboard::Clipboard::new().context("Can't obtain a clipboard handle")?;
    clipboard
        .set_text(value.to_string())
        .context("Unable to set a clipboard contents")
}
