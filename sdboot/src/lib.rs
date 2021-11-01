//! systemd-boot EFI variables manipulation library.

#![deny(missing_docs)]

mod array_ext;
mod error;
mod manager;
mod read;
mod write;

#[cfg(target_os = "linux")]
mod attributes;

pub use manager::Manager;
