use anyhow::{anyhow, Result};
use linapi::system::devices::block::Block;
use parts::types::*;
use std::path::PathBuf;

mod actions;
mod cli;
mod interactive;

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

fn main() -> Result<()> {
    let interactive = cli::handle_args()?;
    if let cli::CliAction::Interactive(info) = interactive {
        eprintln!("Interactive");
        dbg!(&info);
        interactive::handle_tui(info)?;
    }
    Ok(())
}
