use anyhow::Result;
use byte_unit::Byte;
use linapi::system::devices::block::Block;

pub fn get_disks() -> Result<Vec<(String, Block)>> {
    let mut disks = Vec::new();
    for disk in Block::get_connected()? {
        let s = format!(
            "Disk {} - {} - Model: {}",
            disk.name(),
            Byte::from_bytes(disk.size()?.into()).get_appropriate_unit(true),
            disk.model()?.unwrap_or("None".into()),
        );
        disks.push((s, disk));
    }
    Ok(disks)
}
