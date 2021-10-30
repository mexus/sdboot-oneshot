use anyhow::{Context, Result};
use sdboot_oneshot::Manager;
use structopt::StructOpt;

/// A simple utility to manage systemd-boot oneshot entry.
#[derive(Debug, StructOpt)]
struct Args {
    /// Set one shot entry to the provided value.
    #[structopt(long = "set")]
    new_oneshot: Option<String>,

    /// Be verbose.
    #[structopt(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let Args {
        new_oneshot,
        verbose,
    } = Args::from_args();

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
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

    if let Some(new_oneshot) = new_oneshot {
        let mut manager = manager;
        manager.set_oneshot(&new_oneshot)?;
        log::info!(r#"Oneshot entry set to "{}""#, new_oneshot);
        if !entries.contains(&new_oneshot) {
            log::warn!(
                r#"Please note that there is no entry detected with the name "{}"!"#,
                new_oneshot
            )
        }
    }
    Ok(())
}
