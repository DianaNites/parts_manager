use anyhow::{anyhow, Result};
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*},
    views::SelectView,
    Cursive,
};
use linapi::system::devices::block::{Block, Error};
use parts::{types::*, uuid::Uuid, Gpt, PartitionBuilder, PartitionType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use structopt::{
    clap::{arg_enum, AppSettings, Shell},
    StructOpt,
};

#[allow(dead_code)]
mod components;
#[allow(dead_code)]
mod views;

use views::*;

arg_enum! {
    #[derive(Debug, Clone)]
    enum Format {
        Json,
    }
}

#[derive(Clone, Debug, StructOpt)]
#[structopt(global_setting(AppSettings::ColoredHelp))]
struct Args {
    /// Path to device or file.
    #[structopt(
        default_value = "/dev/sda",
        default_value_if("interactive", None, "Auto"),
        required_unless("interactive")
    )]
    device: PathBuf,

    /// Logical Block Size to use. Overrides autodetection from `device`.
    ///
    /// Ignored for `interactive`.
    #[structopt(short, long, global(true))]
    block: Option<u64>,

    /// Use an interactive TUI interface.
    /// If `device` is not specified, displays a selection.
    #[structopt(short, long, conflicts_with("Complete"))]
    interactive: bool,

    #[structopt(subcommand)]
    cmd: Option<Commands>,
}

#[derive(Clone, Debug, StructOpt)]
enum Commands {
    /// Create a new GPT Label.
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
        /// Rounds up to nearest block_size.
        ///
        /// If not specified, uses remaining space.
        #[structopt(long)]
        end: Option<u64>,

        /// Partition type Uuid. Defaults to Linux Filesystem Data
        #[structopt(short, long, default_value = "0FC63DAF-8483-4772-8E79-3D69D8477DE4")]
        partition_type: Uuid,

        /// Partition size, in bytes. Use this OR `end`.
        /// Rounds up to nearest block_size.
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

    /// Dump the GPT Label to disk. Writes to stdout.
    Dump {
        /// Format to output in
        #[structopt(possible_values(&Format::variants()), default_value = "Json")]
        format: Format,
    },

    /// Restore A GPT Label from a previously saved dump to `device`. Reads from
    /// stdin.
    Restore {
        /// Format of dump.
        #[structopt(possible_values(&Format::variants()), default_value = "Json")]
        format: Format,

        /// Whether the `block` option should override the block size in the
        /// dump.
        ///
        /// This flag can be useful if you want to restore the Gpt to a
        /// different disk that has a different block size.
        ///
        /// Only use this if you know what you're doing.
        #[structopt(short, long, requires("block"))]
        override_block: bool,
    },

    /// Generate completions to stdout.
    Complete {
        /// Shell
        #[structopt(possible_values(&Shell::variants()), default_value = "fish")]
        shell: Shell,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct PartitionInfo {
    gpt: Gpt,
    block_size: BlockSize,
    disk_size: Size,
    model: String,
}

fn create_table(
    uuid: Option<Uuid>,
    path: &Path,
    block_size: BlockSize,
    disk_size: Size,
) -> Result<Gpt> {
    let uuid = uuid.unwrap_or_else(Uuid::new_v4);
    let gpt: Gpt = Gpt::new(uuid);
    gpt.to_writer(
        fs::OpenOptions::new().write(true).open(path)?,
        block_size,
        disk_size,
    )?;
    Ok(gpt)
}

#[derive(Debug, Copy, Clone)]
enum End {
    Abs(Offset),
    Rel(Size),
}

fn add_partition(
    gpt: &mut Gpt,
    start: Offset,
    end: End,
    partition_type: Uuid,
    block_size: BlockSize,
    uuid: Uuid,
) -> Result<()> {
    let part = PartitionBuilder::new(uuid)
        .start(start)
        .partition_type(PartitionType::from_uuid(partition_type));
    let part = match end {
        End::Abs(end) => part.end(end),
        End::Rel(size) => part.size(size),
    };
    gpt.add_partition(part.finish(block_size))?;
    //
    Ok(())
}

fn dump(format: Format, info: PartitionInfo) -> Result<String> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(&info)?),
    }
}

#[derive(Debug, Clone)]
struct Info {
    path: PathBuf,
    block_size: BlockSize,
    disk_size: Size,
    model: String,
}

fn get_info_block(block: &Block) -> Result<Info> {
    //
    Ok(Info {
        path: block
            .dev_path()?
            .ok_or_else(|| anyhow!("Couldn't get device file"))?,
        block_size: BlockSize(block.logical_block_size()?),
        disk_size: Size::from_bytes(block.size()?),
        model: block.model()?.unwrap_or_default(),
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
    })
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
            } => match format {
                Format::Json => {
                    let info: PartitionInfo = serde_json::from_reader(std::io::stdin())?;
                    info.gpt.to_writer(
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
            },
            Commands::Complete { shell } => {
                let mut app = Args::clap();
                let name = app.get_name().to_owned();
                app.gen_completions_to(name, shell, &mut std::io::stdout());
            }
        }
    } else {
        //
    }

    //
    Ok(())
}
