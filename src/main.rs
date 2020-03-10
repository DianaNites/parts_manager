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
    ///
    /// If not provided, you can choose between connected block devices.
    device: Option<PathBuf>,

    /// Logical Block Size to use. Ignored when `device` is not specified.
    #[structopt(long, default_value("512"))]
    block: u64,
}

fn main() -> Result<()> {
    let args: Args = Args::from_args();
    //
    let mut root = Cursive::default();
    // Theme
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    root.set_theme(theme);

    // User Entry point
    match args.device {
        Some(path) => {
            let block_size = BlockSize(args.block);
            let disk_size = ByteSize::from_bytes(fs::metadata(&path).unwrap().len());
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            root.add_fullscreen_layer(part_view(&Data::from_path(
                path, name, block_size, disk_size,
            )?)?);
            // Info box will start empty, make sure callback is called and it's set.
            root.call_on_name("parts", |v: &mut SelectView<Partition>| v.set_selection(0))
                .unwrap()(&mut root);
        }
        None => {
            root.add_fullscreen_layer(disks()?);
            // Disk Info box will start empty, make sure callback is called and it's set.
            root.call_on_name("disks", |v: &mut SelectView<Data>| v.set_selection(0))
                .unwrap()(&mut root);
        }
    }
    // if args.device.is_none() {
    //     root.add_fullscreen_layer(disk_selection()?);
    //     // Disk Info box will start empty, make sure callback is called and it's
    // set.     root.call_on_name("disks", |v: &mut SelectView<Block>|
    // v.set_selection(0))         .unwrap()(&mut root);
    // } else {
    //     // partition_view(&mut root);
    // }

    // Global hotkeys
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback('h', |_| todo!("Help menu"));
    //
    root.run();
    //
    Ok(())
}
