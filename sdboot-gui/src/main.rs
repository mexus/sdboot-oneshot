use anyhow::{Context, Result};
use iced::Application;
use sdboot::Manager;
use structopt::{clap::arg_enum, StructOpt};

mod gui;
use gui::GuiApplication;

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

    let mut settings = iced::Settings::default();
    settings.window.size = (600, 300);
    GuiApplication::run(settings).context("Running GUI application")?;

    Ok(())
}
