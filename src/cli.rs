//! Code for the CLI Interface
use crate::{actions::*, Info};
use anyhow::Result;
use parts::types::*;
use std::{ffi::OsStr, fs};
use structopt::StructOpt;
use tracing::{metadata::Metadata, Level};
use tracing_subscriber::{layer, layer::SubscriberExt, FmtSubscriber};

pub mod args;
use args::*;

/// Filter logs based on verbosity.
struct VerboseFilter(bool);

impl layer::Layer<FmtSubscriber> for VerboseFilter {
    fn enabled(&self, _: &Metadata, _: layer::Context<FmtSubscriber>) -> bool {
        self.0
    }
}

/// Handle CLI subcommand actions.
fn handle_cmd(cmd: Commands, info: Info) -> Result<()> {
    match cmd {
        Commands::Create { uuid } => {
            write_gpt_path(&new_gpt(uuid, &info), &info)?;
        }
        Commands::AddPartition {
            start,
            end,
            size,
            partition_type,
            uuid,
        } => {
            let mut f = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&info.path)?;
            let mut gpt = read_gpt(&mut f, &info)?;
            // CLI size, or last partition block + 1, or 1 MiB
            let start = start.map(Offset).unwrap_or_else(|| {
                gpt.partitions()
                    .last()
                    //FIXME: parts API is bad here
                    .map(|p| Offset(p.end().0 + info.block_size.0))
                    .unwrap_or_else(|| Size::from_mib(1).into())
            });
            // If end, absolute. If size, relative. If neither, remaining size.
            let end = match (end, size) {
                (Some(end), None) => End::Abs(Offset(end)),
                (None, Some(size)) => End::Rel(Size::from_bytes(size)),
                (None, None) => End::Rel(gpt.remaining()),
                _ => unreachable!("Clap conflicts prevent this"),
            };
            add_part(&mut gpt, &info, uuid, partition_type, start, end)?;
            write_gpt(&gpt, f, &info)?;
        }
        Commands::Dump { format } => {
            println!("{}", dump(&read_gpt_path(&info)?, format, &info)?);
        }
        Commands::Restore {
            format,
            override_block: _,
        } => {
            // TODO: Version cli argument
            let gpt = restore(format, PartitionInfoVersion::default())?;
            // FIXME: impl override_block. Add block_size to Gpt and then use them here.
            write_gpt_path(&gpt, &info)?;
        }
        Commands::Complete { shell } => {
            let mut app = Args::clap();
            let name = app.get_name().to_owned();
            app.gen_completions_to(name, shell, &mut std::io::stdout());
        }
    }
    //
    Ok(())
}

/// Specifies the action `main` should take.
pub enum CliAction {
    /// All work is done.
    Quit,

    /// Interactive
    Interactive(Option<Info>),
}

/// Handle CLI arguments.
///
/// If interactive, `Interactive(_)` is returned, `Quit` otherwise.
///
/// If interactive AND `device` was specified,
/// `Interactive(Some(Info))`, otherwise `Interactive(None)`.
pub fn handle_args() -> Result<CliAction> {
    let args: Args = Args::from_args();
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(match args.verbose {
                1 => Level::ERROR,
                2 => Level::WARN,
                3 => Level::INFO,
                4 => Level::DEBUG,
                _ => Level::TRACE,
            })
            // completes the builder.
            .finish()
            .with(VerboseFilter(args.verbose != 0)),
    )?;

    if args.cmd.is_some() {
        let info = Info::new_cli(&args)?;
        let cmd = args.cmd.expect("Missing subcommand");
        handle_cmd(cmd, info)?;
        Ok(CliAction::Quit)
    } else if args.interactive {
        if args.device == OsStr::new("Auto") {
            Ok(CliAction::Interactive(None))
        } else {
            let info = Info::new_cli(&args)?;
            Ok(CliAction::Interactive(Some(info)))
        }
    } else {
        unreachable!("Clap requirements should have prevented this")
    }
}
