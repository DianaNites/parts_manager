use anyhow::{anyhow, Result};
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*},
    Cursive,
};
use linapi::system::devices::block::{Block, Error};
use parts::{types::*, uuid::Uuid, Gpt};
use std::{ffi::OsStr, fs, path::PathBuf};
use structopt::StructOpt;

mod cli;
#[allow(dead_code)]
mod components;
#[allow(dead_code)]
mod views;

use cli::{add_partition, create_table, dump, restore, Args, Commands, End, PartitionInfo};
use components::error_quit;
use views::*;

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

fn main() -> Result<()> {
    let args: Args = Args::from_args();
    //
    if args.cmd.is_some() {
        let info = get_info_cli(&args)?;
        let cmd = args.cmd.unwrap();
        //
        let path = info.path;
        let block_size = info.block_size;
        let disk_size = info.disk_size;
        let model = info.model;
        match cmd {
            Commands::Create { uuid } => {
                create_table(uuid, &path, block_size, disk_size)?;
            }
            Commands::AddPartition {
                start,
                end,
                size,
                partition_type,
                uuid,
            } => {
                let mut f = fs::OpenOptions::new().read(true).write(true).open(&path)?;
                let mut gpt: Gpt = Gpt::from_reader(&mut f, block_size, disk_size)?;
                // cmd size, or last partition + block_size, or 1 MiB
                let start = {
                    start.map(Offset).unwrap_or_else(|| {
                        gpt.partitions()
                            .last()
                            .map(|p| (Size::from(p.end()) + block_size).into())
                            .unwrap_or_else(|| Size::from_mib(1).into())
                    })
                };
                let end = match (end, size) {
                    (Some(end), None) => End::Abs(Offset(end)),
                    (None, Some(size)) => End::Rel(Size::from_bytes(size)),
                    (None, None) => todo!("Remaining"),
                    _ => unreachable!("Clap conflicts prevent this"),
                };
                //
                add_partition(
                    &mut gpt,
                    start,
                    end,
                    partition_type,
                    block_size,
                    uuid.unwrap_or_else(Uuid::new_v4),
                )?;
                gpt.to_writer(&mut f, block_size, disk_size)?;
            }
            Commands::Dump { format } => {
                let gpt: Gpt = Gpt::from_reader(fs::File::open(path)?, block_size, disk_size)?;
                let info = PartitionInfo {
                    gpt,
                    block_size,
                    disk_size,
                    model,
                };
                dump(format, info)?;
            }
            Commands::Restore {
                format,
                override_block,
            } => {
                let gpt = restore(format)?;
                gpt.to_writer(
                    fs::OpenOptions::new().write(true).open(path)?,
                    if override_block {
                        assert_ne!(block_size.0, 0);
                        block_size
                    } else {
                        info.block_size
                    },
                    info.disk_size,
                )?;
            }
            Commands::Complete { shell } => {
                let mut app = Args::clap();
                let name = app.get_name().to_owned();
                app.gen_completions_to(name, shell, &mut std::io::stdout());
            }
        }
    } else if args.interactive {
        dbg!(&args);
        //
        let mut root = Cursive::default();
        // Theme
        let mut theme = root.current_theme().clone();
        theme.palette[Background] = TerminalDefault;
        theme.palette[View] = TerminalDefault;
        theme.palette[Primary] = Dark(White);
        theme.palette[Tertiary] = Dark(White);
        root.set_theme(theme);
        // User entry point
        if args.device == OsStr::new("Auto") {
            root.add_fullscreen_layer(disks()?);
            // Disk Info box will start empty, make sure callback is called and it's
            // set.
            root.call_on_name("disks", |v: &mut DiskSelect| v.set_selection(0))
                .unwrap()(&mut root);
        } else {
            let info = get_info_cli(&args)?;
            let gpt: Result<Gpt, _> = Gpt::from_reader(
                fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .open(args.device)?,
                info.block_size,
                info.disk_size,
            );
            match gpt {
                Ok(gpt) => {
                    root.add_fullscreen_layer(parts(gpt));
                }
                Err(e) => {
                    root.add_layer(error_quit(e).button("Create new Gpt?", |root| {
                        //
                        todo!("Create new Gpt")
                    }));
                }
            }
        }
        // Global hotkeys
        root.add_global_callback('q', |s| s.quit());
        root.add_global_callback('h', |_| todo!("Help menu"));
        //
        root.run();
    }

    //
    Ok(())
}
