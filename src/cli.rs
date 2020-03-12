//! Handle CLI stuff
use anyhow::Result;
use parts::{types::*, uuid::Uuid, Gpt, PartitionBuilder, PartitionType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
};
use structopt::{
    clap::{arg_enum, AppSettings, Shell},
    StructOpt,
};

arg_enum! {
    #[derive(Debug, Clone)]
    pub enum Format {
        Json,
    }
}

/// Modern GPT Partition editor
#[derive(Clone, Debug, StructOpt)]
#[structopt(global_settings(&[
    AppSettings::ColoredHelp,
    AppSettings::SubcommandsNegateReqs,
    AppSettings::DisableHelpSubcommand,
    AppSettings::VersionlessSubcommands,
]))]
pub struct Args {
    /// Path to device or file.
    #[structopt(
        default_value = "/dev/sda",
        default_value_if("interactive", None, "Auto"),
        required_unless("interactive")
    )]
    pub device: PathBuf,

    /// Logical Block Size to use. Overrides autodetection from `device`.
    #[structopt(short, long, global(true))]
    pub block: Option<u64>,

    /// Use an interactive TUI interface.
    /// If `device` is not specified, displays a selection.
    #[structopt(short, long, required_unless("subcommand"))]
    pub interactive: bool,

    #[structopt(subcommand)]
    pub cmd: Option<Commands>,
}

#[derive(Clone, Debug, StructOpt)]
pub enum Commands {
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
pub struct PartitionInfo {
    pub gpt: Gpt,
    pub block_size: BlockSize,
    pub disk_size: Size,
    pub model: String,
}

pub fn create_table(
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
pub enum End {
    Abs(Offset),
    Rel(Size),
}

pub fn add_partition(
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

pub fn dump(format: Format, info: PartitionInfo) -> Result<String> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(&info)?),
    }
}

pub fn restore(format: Format) -> Result<Gpt> {
    match format {
        Format::Json => {
            let info: PartitionInfo = serde_json::from_reader(io::stdin())?;
            Ok(info.gpt)
        }
    }
}