//! Code for the CLI Interface
use crate::{actions::*, Info};
use anyhow::{anyhow, Result};
use linapi::system::devices::block::{Block, Error};
use parts::types::*;
use std::{ffi::OsStr, fs};
use structopt::StructOpt;

pub mod args;
use args::*;

/// Get information on a device from CLI args
fn get_info_cli(args: &Args) -> Result<Info> {
    let block = match Block::from_dev(&args.device) {
        Ok(block) => Some(block),
        Err(Error::InvalidArg(_)) => None,
        Err(e) => return Err(e.into()),
    };
    Ok(Info {
        path: args.device.clone(),
        block_size: BlockSize(match args.block {
            Some(s) => s,
            None => {
                // Needed because `block_size` can be None for Restore,
                // and clap will ensure that it's provided if `override_block`
                // is passed.
                //
                // Example cmd: `cargo run -- /tmp/disk2.img restore < /tmp/test`
                // Which MUST work correctly.
                //
                // For other commands we want the default auto behavior.
                if let Some(Commands::Restore { .. }) = args.cmd {
                    0
                } else {
                    block
                        .as_ref() //
                        .ok_or_else(|| {
                            anyhow!("Couldn't automatically determine logical block size")
                        })?
                        .logical_block_size()?
                }
            }
        }),
        disk_size: Size::from_bytes(match block.as_ref() {
            Some(block) => block.size()?,
            None => fs::metadata(&args.device)?.len(),
        }),
        model: match block.as_ref() {
            Some(block) => block.model()?.unwrap_or_default(),
            None => String::new(),
        },
        name: args
            .device
            .file_stem()
            .ok_or_else(|| anyhow!("Invalid device file"))?
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in device file name"))?
            .to_owned(),
    })
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
            // FIXME: Add block_size to Gpt and then use them here.
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
    if args.cmd.is_some() {
        let info = get_info_cli(&args)?;
        let cmd = args.cmd.expect("Missing subcommand");
        handle_cmd(cmd, info)?;
        Ok(CliAction::Quit)
    } else if args.interactive {
        if args.device == OsStr::new("Auto") {
            Ok(CliAction::Interactive(None))
        } else {
            let info = get_info_cli(&args)?;
            Ok(CliAction::Interactive(Some(info)))
        }
    } else {
        unreachable!("Clap requirements should have prevented this")
    }
}
