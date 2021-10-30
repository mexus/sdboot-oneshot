use anyhow::{Context, Result};
use sdboot_oneshot::Manager;
use structopt::StructOpt;

/// A simple utility to manage systemd-boot oneshot entry.
#[derive(Debug, StructOpt)]
struct Args {
    /// Be verbose.
    #[structopt(long, short)]
    verbose: bool,

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
    /// Enter the interactive mode. Short alias is "i".
    #[structopt(alias = "i")]
    Interactive,
}

fn main() -> Result<()> {
    let Args { verbose, command } = Args::from_args();

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
        Some(Command::Interactive) => {
            let mut editor = rustyline::Editor::new();
            editor.set_helper(Some(sdboot_oneshot::interactive::RustylineHelper::new(
                entries,
            )));

            let mut manager = manager;
            loop {
                let input = match editor.readline(">> ") {
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
                    Some("exit") => {
                        // Graceful termination.
                        break;
                    }
                    Some(cmd) => {
                        println!(r#"Unknown command "{}""#, cmd);
                        continue;
                    }
                    None => {
                        println!("No command specified");
                        continue;
                    }
                }
                let entry = match input.next() {
                    Some(entry) => entry,
                    None => {
                        println!("No entry specified");
                        continue;
                    }
                };
                manager.set_oneshot(entry)?;
                log::info!(r#"Oneshot entry set to "{}""#, entry);
            }
        }
        None => { /* No op */ }
    }

    Ok(())
}
