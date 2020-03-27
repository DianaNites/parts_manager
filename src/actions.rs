//! GPT editing actions, interface agnostic.
use crate::Info;
use anyhow::{Context, Result};
use parts::{types::*, uuid::Uuid, Gpt, Partition, PartitionBuilder, PartitionType};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    io::{prelude::*, SeekFrom},
};
use structopt::clap::arg_enum;

arg_enum! {
    /// Supported formats for dumping/restoring the Gpt
    #[derive(Debug, Copy, Clone)]
    pub enum Format {
        Json,
    }
}

/// Either a relative or absolute end. Used by [`add_part`]
#[derive(Debug, Copy, Clone)]
pub enum End {
    Abs(Offset),
    Rel(Size),
}

/// Format versions. Defaults to V1.
#[derive(Debug, Serialize, Deserialize)]
pub enum PartitionInfoVersion {
    /// First version
    V1,
}

/// Defaults to the latest version.
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

/// Portable format to handle Gpt, device, and partitions.
#[derive(Debug, Serialize, Deserialize)]
struct DeviceInfo {
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

/// Dump the Gpt to the portable [`DeviceInfo`] format.
pub fn dump(gpt: &Gpt, format: Format, info: &Info) -> Result<String> {
    let value = DeviceInfo::new(
        &gpt,
        info.block_size,
        info.disk_size,
        // FIXME: No actual need to clone here.
        info.model.clone(),
        Default::default(),
    );
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(&value)?),
    }
}

/// Restore the Gpt from the portable [`DeviceInfo`] format.
// FIXME: To minimal, can do invalid restores? Bigger function?
pub fn restore(format: Format, _version: PartitionInfoVersion) -> Result<Gpt> {
    match format {
        Format::Json => {
            let info: DeviceInfo = serde_json::from_reader(io::stdin())?;
            Ok(info.into_gpt()?)
        }
    }
}

/// Create and return a new empty Gpt.
pub fn new_gpt<U: Into<Option<Uuid>>>(uuid: U, info: &Info) -> Gpt {
    Gpt::new(
        uuid.into().unwrap_or_else(Uuid::new_v4),
        info.disk_size,
        info.block_size,
    )
}

/// Read and return the Gpt from `source`.
pub fn read_gpt<R: Read + Seek>(source: R, info: &Info) -> Result<Gpt> {
    Ok(Gpt::from_reader(source, info.block_size)?)
}

/// Read the Gpt from `path`
pub fn read_gpt_path(info: &Info) -> Result<Gpt> {
    let source = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&info.path)
        .with_context(|| format!("Couldn't open {}", info.path.display()))?;
    read_gpt(source, info)
}

/// Add a partition to the Gpt.
pub fn add_part<U>(
    gpt: &mut Gpt,
    info: &Info,
    uuid: U,
    partition_type: Uuid,
    start: Offset,
    end: End,
) -> Result<()>
where
    U: Into<Option<Uuid>>,
{
    let part = PartitionBuilder::new(uuid.into().unwrap_or_else(Uuid::new_v4))
        .start(start)
        .partition_type(PartitionType::from_uuid(partition_type));
    let part = match end {
        End::Abs(end) => part.end(end),
        End::Rel(size) => part.size(size),
    };
    gpt.add_partition(part.finish(info.block_size))?;
    Ok(())
}

/// Write the Gpt to `dest`.
pub fn write_gpt<W: Write + Seek>(gpt: &Gpt, mut dest: W, info: &Info) -> Result<()> {
    gpt.to_bytes_with_func(
        |i, buf| {
            dest.seek(SeekFrom::Start(i.0))?;
            dest.write_all(buf)?;
            Ok(())
        },
        info.block_size,
        info.disk_size,
    )?;
    Ok(())
}

/// Write the Gpt to `path`
pub fn write_gpt_path(gpt: &Gpt, info: &Info) -> Result<()> {
    let dest = fs::OpenOptions::new()
        .write(true)
        .open(&info.path)
        .with_context(|| format!("Couldn't create {}", info.path.display()))?;
    write_gpt(gpt, dest, info)?;
    Ok(())
}
