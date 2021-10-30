use std::sync::Arc;

use iced::{button, Button, Column, Container, Length, Radio, Row, Sandbox, Text};

use crate::Manager;

/// GUI application.
pub struct GuiApplication {
    manager: Manager,
    entries: Arc<[Arc<str>]>,
    selected: Option<Arc<str>>,
    apply_button_state: button::State,
    unset_button_state: button::State,
    message: String,
}

/// Application message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    RadioSelect { new_value: Arc<str> },
    Unset,
    Apply,
}

impl Sandbox for GuiApplication {
    type Message = Message;

    fn new() -> Self {
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
            apply_button_state: button::State::new(),
            unset_button_state: button::State::new(),
            message: String::new(),
        }
    }

    fn title(&self) -> String {
        "Systemd-boot oneshot entries manager".to_owned()
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::RadioSelect { new_value } => self.selected = Some(new_value),
            Message::Unset => {
                log::info!("Removing oneshot entry");
                if let Err(e) = self.manager.remove_oneshot() {
                    log::error!("Unable to remove oneshot entry: {:#}", e);
                    self.message = format!("Unable to remove oneshot entry: {:#}", e);
                } else {
                    self.message = "Oneshot entry unset".to_string();
                    self.selected = None;
                }
            }
            Message::Apply => {
                if let Some(selected) = self.selected.as_ref() {
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
        }
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        let title = Text::new("Boot entries");

        let radios = self.entries.iter().map(|entry| {
            Radio::new(entry, entry as &str, self.selected.as_ref(), |entry| {
                Message::RadioSelect {
                    new_value: Arc::clone(entry),
                }
            })
        });

        let mut entries = Column::new()
            .width(Length::Fill)
            .spacing(10)
            .align_items(iced::Align::Start);
        for radio in radios {
            entries = entries.push(radio);
        }

        let apply_button =
            Button::new(&mut self.apply_button_state, Text::new("Apply")).on_press(Message::Apply);

        let unset_button =
            Button::new(&mut self.unset_button_state, Text::new("Unset")).on_press(Message::Unset);

        let buttons = Row::new()
            .width(Length::Fill)
            .spacing(20)
            .align_items(iced::Align::Center)
            .push(unset_button)
            .push(apply_button);

        let mut content = Column::new()
            .width(Length::Units(500))
            .spacing(20)
            .align_items(iced::Align::Center)
            .push(title)
            .push(entries)
            .push(buttons);

        if !self.message.is_empty() {
            content = content.push(Text::new(&self.message));
        }

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .center_x()
            .center_y()
            .into()
    }
}
