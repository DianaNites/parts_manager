//! CLI Argument handling code
use crate::actions::Format;
use anyhow::{anyhow, Result};
use parts::{types::Size, uuid::Uuid};
use std::path::PathBuf;
use structopt::{
    clap::{AppSettings, Shell},
    StructOpt,
};

// TODO: Have parts::types::Size impl FromStr and use that instead.
fn parse_size(arg: &str) -> Result<u64> {
    let arg = arg.trim();
    // Index of first non digit, or arg len.
    let idx = arg
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or_else(|| arg.len());
    let num: u64 = arg[..idx].trim().parse()?;
    let suf = arg[idx..].trim().to_ascii_uppercase();

    match &*suf {
        "" => Ok(num),
        "K" => Ok(Size::from_kib(num).as_bytes()),
        "M" => Ok(Size::from_mib(num).as_bytes()),
        "G" => Ok(Size::from_gib(num).as_bytes()),
        "T" => Ok(Size::from_tib(num).as_bytes()),
        _ => Err(anyhow!("Invalid suffix")),
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
        global(true)
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
    #[structopt(alias("add"))]
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

        /// Partition size, in bytes.
        ///
        /// You can use the KiB, MiB, GiB, and TiB suffixes here.
        /// (The `iB` is optional)
        ///
        /// Note that partitions can only be specified in terms of the
        /// logical block size, so this value may be rounded up.
        ///
        /// If not specified, uses remaining space.
        #[structopt(long, conflicts_with("end"), parse(try_from_str = parse_size))]
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
        #[structopt(long, case_insensitive(true), possible_values(&Format::variants()), default_value = "Json")]
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
