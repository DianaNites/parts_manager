use anyhow::Result;
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*},
    views::SelectView,
    Cursive,
};
use parts::{
    types::{BlockSize, ByteSize},
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
    #[structopt(long)]
    block: Option<u64>,
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
    let block_size = BlockSize(args.block.unwrap());
    let disk_size = ByteSize::from_bytes(fs::metadata(&path).unwrap().len());
    let name = path.file_name().unwrap().to_str().unwrap().to_owned();
    //
    let data = Data::from_path(path, name, block_size, disk_size)?;
    //
    Ok(())
}
