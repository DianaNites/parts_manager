//! Code for the CLI Interface
use anyhow::Result;
use parts::{types::*, uuid::Uuid, Gpt, Partition, PartitionBuilder, PartitionType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    io::{prelude::*, SeekFrom},
    path::Path,
};

pub mod args;

// FIXME: Remove this
pub use args::*;

/// Format versions. Defaults to V1.
#[derive(Debug, Serialize, Deserialize)]
pub enum PartitionInfoVersion {
    /// First version
    V1,
}

impl Default for PartitionInfoVersion {
    fn default() -> Self {
        PartitionInfoVersion::V1
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PartInfo {
    name: String,
    part_type: PartitionType,
    uuid: Uuid,
    start: Offset,
    end: Offset,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionInfo {
    #[serde(default)]
    version: PartitionInfoVersion,

    uuid: Uuid,

    model: String,

    block_size: BlockSize,

    device_size: Size,

    partitions: Vec<PartInfo>,
}

impl PartitionInfo {
    pub fn new(
        gpt: &Gpt,
        block_size: BlockSize,
        device_size: Size,
        model: String,
        version: PartitionInfoVersion,
    ) -> Self {
        PartitionInfo {
            version,
            model,
            uuid: gpt.uuid(),
            block_size,
            device_size,
            partitions: gpt
                .partitions()
                .iter()
                .map(|p: &Partition| PartInfo {
                    name: p.name().into(),
                    part_type: p.partition_type(),
                    uuid: p.uuid(),
                    start: p.start(),
                    end: p.end(),
                })
                .collect(),
        }
    }

    pub fn into_gpt(self) -> Result<Gpt> {
        let mut gpt = Gpt::new(self.uuid, self.device_size, self.block_size);
        for part in self.partitions {
            let part = PartitionBuilder::new(part.uuid)
                .name(&part.name)
                .partition_type(part.part_type)
                .start(part.start)
                .end(part.end)
                .finish(self.block_size);
            gpt.add_partition(part)?;
        }
        Ok(gpt)
    }
}

pub fn create_table(
    uuid: Option<Uuid>,
    path: &Path,
    block_size: BlockSize,
    disk_size: Size,
) -> Result<Gpt> {
    let uuid = uuid.unwrap_or_else(Uuid::new_v4);
    let gpt: Gpt = Gpt::new(uuid, disk_size, block_size);
    let mut f = fs::OpenOptions::new().write(true).open(path)?;
    gpt.to_bytes_with_func(
        |o, buf| {
            f.seek(SeekFrom::Start(o.0))?;
            f.write_all(buf)?;
            Ok(())
        },
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
            Ok(info.into_gpt()?)
        }
    }
}
