use anyhow::{anyhow, Result};
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*},
    views::SelectView,
    Cursive,
};
use linapi::system::devices::block::{Block, Error};
use parts::{
    types::{BlockSize, ByteSize},
    uuid::Uuid,
    Gpt,
    Partition,
};
use std::{fs, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};

mod components;
mod views;

use views::*;

#[derive(Debug, StructOpt)]
#[structopt(global_setting(AppSettings::ColoredHelp))]
struct Args {
    /// Path to device or file to partition.
    #[structopt(default_value = "/dev/sda")]
    device: PathBuf,

    /// Logical Block Size to use. Overrides autodetection from `device`.
    #[structopt(short, long)]
    block: Option<u64>,

    #[structopt(subcommand)]
    cmd: Commands,
}

#[derive(Debug, StructOpt)]
enum Commands {
    /// Create a new GPT Label
    Create {
        /// Use this specific UUID instead of generating a new one.
        ///
        /// WARNING: Gpt UUID's must be unique.
        /// Only use this if you know what you're doing.
        uuid: Option<Uuid>,
    },
}

fn interactive() -> Result<()> {
    let mut root = Cursive::default();
    // Theme
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    root.set_theme(theme);

    root.add_fullscreen_layer(disks()?);
    // Disk Info box will start empty, make sure callback is called and it's set.
    root.call_on_name("disks", |v: &mut SelectView<Data>| v.set_selection(0))
        .unwrap()(&mut root);

    // Global hotkeys
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback('h', |_| todo!("Help menu"));
    //
    root.run();
    Ok(())
}

fn main() -> Result<()> {
    let args: Args = Args::from_args();
    //
    let path = args.device;
    let block = match Block::from_dev(&path) {
        Ok(block) => Some(block),
        Err(Error::Invalid) => None,
        Err(e) => return Err(e.into()),
    };
    dbg!(&block);
    let block_size = match args.block {
        Some(s) => s,
        None => block
            .as_ref()
            .map(|b| b.logical_block_size())
            .ok_or_else(|| anyhow!("Couldn't automatically logical block size"))??,
    };
    dbg!(block_size);
    let file_size = match block.as_ref() {
        Some(block) => block.size()?,
        None => fs::metadata(&path)?.len(),
    };
    dbg!(file_size);
    let name = match block.as_ref() {
        Some(block) => block.name().to_owned(),
        None => path
            .file_name()
            .ok_or_else(|| anyhow!("Missing filename"))?
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in filename"))?
            .to_owned(),
    };
    dbg!(&name);
    let block_size = BlockSize(block_size);
    dbg!(block_size);
    let disk_size = ByteSize::from_bytes(file_size);
    dbg!(disk_size);
    // let gpt = Gpt::from_reader(fs::File::open(path)?, block_size, disk_size)?;
    // dbg!(&gpt);
    match args.cmd {
        Commands::Create { uuid } => {
            let uuid = uuid.unwrap_or_else(|| Uuid::new_v4());
            let gpt = Gpt::new();
            gpt.to_writer(
                fs::OpenOptions::new().write(true).open(path)?,
                block_size,
                disk_size,
            )?;
        }
    }
    //
    Ok(())
}
