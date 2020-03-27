//! GPT editing actions, interface agnostic.
use anyhow::{Context, Result};
use parts::{types::*, uuid::Uuid, Gpt, Partition, PartitionBuilder, PartitionType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
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
pub struct DeviceInfo {
    #[serde(default)]
    version: PartitionInfoVersion,

    uuid: Uuid,

    model: String,

    block_size: BlockSize,

    device_size: Size,

    partitions: Vec<PartInfo>,
}

impl DeviceInfo {
    pub fn new(
        gpt: &Gpt,
        block_size: BlockSize,
        device_size: Size,
        model: String,
        version: PartitionInfoVersion,
    ) -> Self {
        DeviceInfo {
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

pub fn dump(format: Format, info: DeviceInfo) -> Result<String> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(&info)?),
    }
}

pub fn restore(format: Format) -> Result<Gpt> {
    match format {
        Format::Json => {
            let info: DeviceInfo = serde_json::from_reader(io::stdin())?;
            Ok(info.into_gpt()?)
        }
    }
}
