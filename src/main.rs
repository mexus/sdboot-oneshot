use std::os::unix::prelude::AsRawFd;

use anyhow::{Context, Result};
use fern::colors::{Color, ColoredLevelConfig};
use sdboot_oneshot::Manager;
use structopt::{clap::arg_enum, StructOpt};

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

    /// Set the color mode.
    #[structopt(
        long = "color", default_value = "auto",
        possible_values = &ColorMode::variants(),
        case_insensitive = true,
    )]
    color_mode: ColorMode,

    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Set one shot entry. Short alias is "s".
    #[structopt(alias = "s")]
    Set {
        /// New one shot entry name.
        entry: String,
    },

    #[cfg(target_os = "linux")]
    /// Removes the one shot entry.
    Unset,

    /// Enter the interactive mode. Short alias is "i".
    #[structopt(alias = "i")]
    Interactive,
}

fn main() -> Result<()> {
    let Args {
        verbose,
        command,
        color_mode,
    } = Args::from_args();

    let colorful_logs = match color_mode {
        ColorMode::Auto => nix::unistd::isatty(std::io::stdout().as_raw_fd()).unwrap_or(false),
        ColorMode::On => true,
        ColorMode::Off => false,
    };

    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Cyan);
    let formatter: Box<
        dyn Fn(fern::FormatCallback, &std::fmt::Arguments, &log::Record) + Sync + Send,
    > = if colorful_logs {
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

    let manager = Manager::new();

    log::info!(r#"Default entry: "{}""#, manager.get_default_entry()?);
    log::info!(r#"Currently booted: "{}""#, manager.get_entry_selected()?);

    if let Some(current_oneshot_entry) = manager.get_oneshot()? {
        log::info!(
            r#"One shot is currently set to "{}""#,
            current_oneshot_entry
        );
    } else {
        log::info!(r#"One shot is currently not set"#);
    }

    let entries = manager.entries()?;
    log::info!("Discovered {} entries: {:#?}", entries.len(), entries);

    match command {
        Some(Command::Set { entry }) => {
            let mut manager = manager;
            manager.set_oneshot(&entry)?;
            log::info!(r#"Oneshot entry set to "{}""#, entry);
            if !entries.contains(&entry) {
                log::warn!(
                    r#"Please note that there is no entry detected with the name "{}"!"#,
                    entry
                )
            }
        }
        #[cfg(target_os = "linux")]
        Some(Command::Unset) => manager.remove_oneshot()?,
        Some(Command::Interactive) => {
            let mut editor = rustyline::Editor::new();
            editor.set_helper(Some(sdboot_oneshot::interactive::RustylineHelper::new(
                entries.clone(),
            )));

            let mut manager = manager;

            let prompt = if colorful_logs {
                format!(
                    "{color}>>\x1B[0m ",
                    color = format_args!("\x1B[{}m", Color::BrightGreen.to_fg_str())
                )
            } else {
                ">> ".to_owned()
            };
            loop {
                let input = match editor.readline(&prompt) {
                    Ok(input) => input,
                    Err(rustyline::error::ReadlineError::Eof) => {
                        // Graceful termination
                        break;
                    }
                    Err(e) => return Err(e).context("Input error"),
                };
                editor.add_history_entry(&input);
                let mut input = input.split(char::is_whitespace).filter(|s| !s.is_empty());
                match input.next() {
                    Some("set") => { /* No op */ }
                    #[cfg(target_os = "linux")]
                    Some("unset") => {
                        if let Err(e) = manager.remove_oneshot() {
                            log::error!("Unable to remove oneshot: {:#}", e)
                        }
                        continue;
                    }
                    Some("exit") => {
                        // Graceful termination.
                        break;
                    }
                    Some(cmd) => {
                        log::warn!(r#"Unknown command "{}""#, cmd);
                        continue;
                    }
                    None => {
                        log::warn!("No command specified");
                        continue;
                    }
                }
                let entry = match input.next() {
                    Some(entry) => entry,
                    None => {
                        log::warn!("No entry specified");
                        continue;
                    }
                };
                if let Err(e) = manager.set_oneshot(entry) {
                    log::error!("Unable to set oneshot: {:#}", e);
                    continue;
                }
                log::info!(r#"Oneshot entry set to "{}""#, entry);
                if !entries.iter().any(|existing| existing == entry) {
                    log::warn!(
                        r#"Please note that there is no entry detected with the name "{}"!"#,
                        entry
                    );
                }
            }
        }
        None => { /* No op */ }
    }

    Ok(())
}
