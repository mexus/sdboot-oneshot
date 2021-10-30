//! systemd-boot EFI variables manipulation library.

#![deny(missing_docs)]

mod array_ext;
mod error;
mod gui;
pub mod interactive;
mod manager;
mod read;
mod write;

#[cfg(target_os = "linux")]
mod attributes;

pub use gui::GuiApplication;
pub use manager::Manager;
