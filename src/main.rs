use std::{os::unix::prelude::AsRawFd, sync::Arc};

use anyhow::{Context, Result};
use fern::colors::{Color, ColoredLevelConfig};
use iced::Application;
use sdboot_oneshot::{GuiApplication, Manager};
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

    /// Removes the one shot entry.
    Unset,

    /// Enter the interactive mode. Short alias is "i".
    #[structopt(alias = "i")]
    Interactive,

    /// Launch graphical user interface. Short alias is "g".
    #[structopt(alias = "g")]
    Gui,
}

fn load_entries(manager: &Manager) -> Result<Arc<[String]>> {
    use once_cell::sync::OnceCell;
    static ENTRIES: OnceCell<Arc<[String]>> = OnceCell::new();

    ENTRIES
        .get_or_try_init(|| manager.entries().map(Arc::from))
        .map(Arc::clone)
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
        } else if cfg!(windows) && matches!(command, Some(Command::Gui)) {
            // Do not log anything on windows when in GUI mode.
            log::LevelFilter::Off
        } else {
            log::LevelFilter::Info
        })
        .chain(std::io::stdout())
        .apply()
        .context("Unable to initialize logging")?;

    let mut manager = Manager::new();

    if !matches!(command, Some(Command::Gui)) {
        log::info!(r#"Default entry: "{}""#, manager.get_default_entry()?);
        log::info!(r#"Currently booted: "{}""#, manager.get_selected_entry()?);

        if let Some(current_oneshot_entry) = manager.get_oneshot()? {
            log::info!(
                r#"One shot is currently set to "{}""#,
                current_oneshot_entry
            );
        } else {
            log::info!(r#"One shot is currently not set"#);
        }

        let entries = load_entries(&manager)?;
        log::info!("Discovered {} entries: {:#?}", entries.len(), entries);
    }

    match command {
        Some(Command::Set { entry }) => {
            let entries = load_entries(&manager)?;
            manager.set_oneshot(&entry)?;
            log::info!(r#"Oneshot entry set to "{}""#, entry);
            if !entries.contains(&entry) {
                log::warn!(
                    r#"Please note that there is no entry detected with the name "{}"!"#,
                    entry
                )
            }
        }
        Some(Command::Unset) => manager.remove_oneshot()?,
        Some(Command::Interactive) => {
            let entries = load_entries(&manager)?;
            let mut editor = rustyline::Editor::new();
            editor.set_helper(Some(sdboot_oneshot::interactive::RustylineHelper::new(
                entries,
            )));

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
                match load_entries(&manager) {
                    Ok(entries) => {
                        if !entries.iter().any(|existing| existing == entry) {
                            log::warn!(
                                r#"Please note that there is no entry detected with the name "{}"!"#,
                                entry
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("Unable to load available entries: {:#}", e);
                    }
                };
            }
        }
        Some(Command::Gui) => {
            let mut settings = iced::Settings::default();
            settings.window.size = (600, 300);
            GuiApplication::run(settings).context("Running GUI application")?;
        }
        None => { /* No op */ }
    }

    Ok(())
}
