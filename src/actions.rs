//! GPT editing actions, interface agnostic.
use anyhow::{Context, Result};
use parts::{types::*, uuid::Uuid, Gpt};
use std::{
    fs,
    io::{prelude::*, SeekFrom},
    path::Path,
};
use structopt::clap::arg_enum;

arg_enum! {
    /// Supported formats for dumping/restoring the Gpt
    #[derive(Debug, Copy, Clone)]
    pub enum Format {
        Json,
    }
}

/// Create and write a new empty Gpt
pub fn create_table(
    uuid: Option<Uuid>,
    path: &Path,
    block_size: BlockSize,
    disk_size: Size,
) -> Result<Gpt> {
    let uuid = uuid.unwrap_or_else(Uuid::new_v4);
    let gpt: Gpt = Gpt::new(uuid, disk_size, block_size);
    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("Couldn't create file {}", path.display()))?;
    gpt.to_bytes_with_func(
        |o, buf| {
            f.seek(SeekFrom::Start(o.0))?;
            f.write_all(buf)?;
            Ok(())
        },
        block_size,
        disk_size,
    )
    .with_context(|| format!("Couldn't write GPT to {}", path.display()))?;
    Ok(gpt)
}
