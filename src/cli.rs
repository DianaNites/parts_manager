//! Code for the CLI Interface
use anyhow::Result;
use parts::{types::*, uuid::Uuid, Gpt, PartitionBuilder, PartitionType};

pub mod args;

// FIXME: Remove this
pub use args::*;

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
