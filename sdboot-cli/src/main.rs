use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fern::colors::{Color, ColoredLevelConfig};
use sdboot::Manager;

#[derive(PartialEq, Debug, clap::ValueEnum, Clone, Copy)]
pub enum ColorMode {
    Auto,
    On,
    Off,
}

/// A simple utility to manage systemd-boot oneshot entry.
#[derive(Parser)]
struct Args {
    /// Be verbose.
    #[clap(long, short)]
    verbose: bool,

    /// Set the color mode.
    #[clap(value_enum, long = "color", default_value_t = ColorMode::Auto)]
    color_mode: ColorMode,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Set one shot entry. Short alias is "so".
    #[clap(name = "set-oneshot", alias = "so")]
    SetOneshot {
        /// New one shot entry name.
        entry: String,
    },

    /// Set default entry. Short alias is "sd".
    #[clap(name = "set-default", alias = "sd")]
    SetDefault {
        /// New default entry name.
        entry: String,
    },

    /// Removes the one shot entry.
    Unset,
}

fn main() -> Result<()> {
    let Args {
        verbose,
        command,
        color_mode,
    } = Args::parse();

    let colorful_logs = match color_mode {
        ColorMode::Auto => {
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::io::AsRawFd;
                nix::unistd::isatty(std::io::stdout().as_raw_fd()).unwrap_or(false)
            }
            #[cfg(not(target_os = "linux"))]
            {
                true
            }
        }
        ColorMode::On => true,
        ColorMode::Off => false,
    };

    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Cyan);
    type Formatter =
        Box<dyn Fn(fern::FormatCallback, &std::fmt::Arguments, &log::Record) + Sync + Send>;
    let formatter: Formatter = if colorful_logs {
        Box::new(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}{message}\x1B[0m",
                color_line =
                    format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                message = message
            ))
        })
    } else {
        Box::new(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.level(), message))
        })
    };

    fern::Dispatch::new()
        .format(formatter)
        .level(if verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(std::io::stdout())
        .apply()
        .context("Unable to initialize logging")?;

    let mut manager = Manager::new();

    if let Some(name) = manager.get_default_entry()? {
        log::info!(r#"Default entry: "{name}""#);
    } else {
        log::info!("Default entry: not set");
    }

    if let Some(name) = manager.get_selected_entry()? {
        log::info!(r#"Currently booted: "{name}""#);
    } else {
        log::info!(r#"Currently booted: not booted with systemd-boot"#);
    }

    if let Some(current_oneshot_entry) = manager.get_oneshot()? {
        log::info!(
            r#"One shot is currently set to "{}""#,
            current_oneshot_entry
        );
    } else {
        log::info!(r#"One shot is currently not set"#);
    }

    let entries = manager.entries().context("Unable to fetch entries")?;
    log::info!("Discovered {} entries: {:#?}", entries.len(), entries);

    match command {
        Some(Command::SetOneshot { entry }) => {
            manager.set_oneshot(&entry)?;
            log::info!(r#"Oneshot entry set to "{}""#, entry);
            if !entries.contains(&entry) {
                log::warn!(
                    r#"Please note that there is no entry detected with the name "{}"!"#,
                    entry
                )
            }
        }
        Some(Command::SetDefault { entry }) => {
            manager.set_default(&entry)?;
            log::info!(r#"Default entry set to "{}""#, entry);
            if !entries.contains(&entry) {
                log::warn!(
                    r#"Please note that there is no entry detected with the name "{}"!"#,
                    entry
                )
            }
        }
        Some(Command::Unset) => {
            manager.remove_oneshot()?;
            log::info!("Oneshot entry unset");
        }
        None => { /* No op */ }
    }

    Ok(())
}
