use anyhow::{anyhow, Result};
use cursive::Cursive;
use linapi::system::devices::block::{Block, Error};
use parts::{types::*, uuid::Uuid, Gpt};
use std::{ffi::OsStr, fs, path::PathBuf};
use structopt::StructOpt;

mod actions;
mod cli;
mod interactive;

use cli::args::{Args, Commands};
use interactive::{components::error_quit, views::*};

#[derive(Debug, Clone)]
pub struct Info {
    pub path: PathBuf,
    pub block_size: BlockSize,
    pub disk_size: Size,
    pub model: String,
    pub name: String,
}

pub fn get_info_block(block: &Block) -> Result<Info> {
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

fn get_info_cli(args: &Args) -> Result<Info> {
    let block = match Block::from_dev(&args.device) {
        Ok(block) => Some(block),
        Err(Error::Invalid) => None,
        Err(e) => return Err(e.into()),
    };
    Ok(Info {
        path: args.device.clone(),
        block_size: BlockSize(match args.block {
            Some(s) => s,
            None => {
                if let Some(Commands::Restore { .. }) = args.cmd {
                    0
                } else {
                    block
                        .as_ref()
                        .map(|b| b.logical_block_size())
                        .ok_or_else(|| {
                            anyhow!("Couldn't automatically determine logical block size")
                        })??
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

#[allow(unreachable_code)]
fn main() -> Result<()> {
    let interactive = cli::handle_args()?;
    if interactive {
        interactive::handle_tui()?;
    }
    return Ok(());
    //
    let args: Args = Args::from_args();
    //
    if args.interactive {
        let mut root = Cursive::default();
        // User entry point
        if args.device == OsStr::new("Auto") {
            root.add_fullscreen_layer(disks()?);
            setup_views(&mut root);
        } else {
            let info = get_info_cli(&args)?;
            let gpt: Result<Gpt, _> = Gpt::from_reader(
                fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .open(args.device)?,
                info.block_size,
            );
            match gpt {
                Ok(gpt) => {
                    root.add_fullscreen_layer(parts(gpt, &info));
                    setup_views(&mut root);
                }
                Err(e) => {
                    root.add_layer(error_quit(e).button("New Gpt", move |mut root| {
                        let gpt: Gpt = Gpt::new(Uuid::new_v4(), info.disk_size, info.block_size);
                        root.pop_layer();
                        root.add_fullscreen_layer(parts(gpt, &info));
                        setup_views(&mut root);
                    }));
                }
            };
        }
        // Global hotkeys
        root.add_global_callback('q', |s| s.quit());
        root.add_global_callback('h', |_| todo!("Help menu"));
        // Required for parts, it'll start unset if no partitions
        root.set_user_data(None::<parts::Partition>);
        //
        root.run();
    }

    //
    Ok(())
}
