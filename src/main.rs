use anyhow::{anyhow, Result};
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*},
    views::SelectView,
    Cursive,
};
use linapi::system::devices::block::{Block, Error};
use parts::{types::*, uuid::Uuid, Gpt, PartitionBuilder, PartitionType};
use std::{fs, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};

#[allow(dead_code)]
mod components;
#[allow(dead_code)]
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
    ///
    /// WARNING: This WILL IMMEDIATELY overwrite ANY existing Gpt
    Create {
        /// Use this specific UUID instead of generating a new one.
        ///
        /// WARNING: Gpt UUID's must be unique.
        /// Only use this if you know what you're doing.
        #[structopt(long)]
        uuid: Option<Uuid>,
    },

    /// Add a partition to the Gpt.
    AddPartition {
        /// Partition start, in bytes.
        ///
        /// If not specified, partition starts after last existing partition,
        /// or at 1 MiB.
        #[structopt(long)]
        start: Option<u64>,

        /// Partition end, in bytes. Inclusive.
        ///
        /// If not specified, uses remaining space.
        #[structopt(long)]
        end: Option<u64>,

        /// Partition type Uuid. Defaults to Linux Filesystem Data
        #[structopt(short, long, default_value = "0FC63DAF-8483-4772-8E79-3D69D8477DE4")]
        partition_uuid: Uuid,

        /// Partition size, in bytes. Use this OR `end`.
        ///
        /// If not specified, uses remaining space.
        #[structopt(long, conflicts_with("end"))]
        size: Option<u64>,

        /// Use this specific UUID instead of generating a new one.
        ///
        /// WARNING: Partition UUID's must be unique.
        /// Only use this if you know what you're doing.
        #[structopt(long)]
        uuid: Option<Uuid>,
    },
}

#[allow(dead_code)]
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
    let disk_size = Size::from_bytes(file_size);
    dbg!(disk_size);
    match args.cmd {
        Commands::Create { uuid } => {
            let _uuid = uuid.unwrap_or_else(Uuid::new_v4);
            let gpt = Gpt::new();
            gpt.to_writer(
                fs::OpenOptions::new().write(true).open(path)?,
                block_size,
                disk_size,
            )?;
        }
        Commands::AddPartition {
            start,
            end,
            size,
            partition_uuid,
            uuid: _,
        } => {
            let mut f = fs::OpenOptions::new().read(true).write(true).open(path)?;
            let mut gpt = Gpt::from_reader(&mut f, block_size, disk_size)?;
            // cmd size, or last partition + block_size, or 1 MiB
            let start = {
                start.map(Offset).unwrap_or_else(|| {
                    gpt.partitions()
                        .last()
                        .map(|p| (Size::from(p.end()) + block_size).into())
                        .unwrap_or_else(|| Size::from_mib(1).into())
                })
            };
            dbg!(start);
            let part = PartitionBuilder::new(Uuid::new_v4())
                .start(start)
                .partition_type(PartitionType::from_uuid(partition_uuid));
            let part = match (end, size) {
                (Some(end), None) => part.end(Size::from_bytes(end).into()),
                (None, Some(size)) => part.size(Size::from_bytes(size)),
                (None, None) => todo!("Remaining"),
                _ => unreachable!("Clap conflicts prevent this"),
            };
            gpt.add_partition(part.finish(block_size))?;
            //
            gpt.to_writer(&mut f, block_size, disk_size)?;
        }
    }
    //
    Ok(())
}
