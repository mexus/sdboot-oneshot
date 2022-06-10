#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::{Context, Result};
use egui::Vec2;
use sdboot::Manager;
use structopt::{clap::arg_enum, StructOpt};

mod gui;

arg_enum! {
    #[derive(PartialEq, Debug)]
    pub enum ColorMode {
        Auto,
        On,
        Off,
    }
}

/// A simple utility to manage systemd-boot oneshot entry.
#[derive(Debug, StructOpt)]
struct Args {
    /// Be verbose.
    #[structopt(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let Args { verbose } = Args::from_args();

    fern::Dispatch::new()
        .format(|out, message, record| out.finish(format_args!("[{}] {}", record.level(), message)))
        .level(if verbose {
            log::LevelFilter::Debug
        } else if cfg!(windows) {
            // Do not log anything on windows when in GUI mode.
            log::LevelFilter::Off
        } else {
            log::LevelFilter::Info
        })
        .chain(std::io::stdout())
        .apply()
        .context("Unable to initialize logging")?;

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(400., 200.)),
        ..Default::default()
    };
    eframe::run_native(
        "Systemd-boot oneshot entries manager",
        native_options,
        Box::new(|_cc| Box::new(gui::GuiApplication::default())),
    );
}
