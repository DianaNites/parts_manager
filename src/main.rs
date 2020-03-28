use anyhow::{anyhow, Result};
use linapi::system::devices::block::{Block, Error};
use parts::types::*;
use std::{fs, path::PathBuf};

mod actions;
mod cli;
mod interactive;

#[derive(Debug, Clone)]
pub struct Info {
    pub path: PathBuf,
    pub block_size: BlockSize,
    pub disk_size: Size,
    pub model: String,
    pub name: String,
}

impl Info {
    /// Get information on a device from CLI args
    pub fn new_cli(args: &cli::args::Args) -> Result<Info> {
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
                    if let Some(cli::args::Commands::Restore { .. }) = args.cmd {
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

    /// Get information on a device from a [`Block`]
    pub fn new_block(block: &Block) -> Result<Info> {
        Ok(Info {
            path: block
                .dev_path()?
                .ok_or_else(|| anyhow!("Couldn't get device file"))?,
            block_size: BlockSize(block.logical_block_size()?),
            disk_size: Size::from_bytes(block.size()?),
            model: block.model()?.unwrap_or_default(),
            name: block.name().to_owned(),
        })
    }
}

fn main() -> Result<()> {
    let interactive = cli::handle_args()?;
    if let cli::CliAction::Interactive(info) = interactive {
        interactive::handle_tui(info)?;
    }
    Ok(())
}
