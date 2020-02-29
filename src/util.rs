use byte_unit::Byte;
use linapi::system::devices::block::Block;
use std::fs;

pub fn get_disks() -> Vec<(String, Block)> {
    Block::get_connected()
        .unwrap()
        .into_iter()
        .map(|d| {
            (
                format!(
                    "Disk {} - {} - Model: {}",
                    d.name(),
                    Byte::from_bytes(d.size().unwrap().into()).get_appropriate_unit(true),
                    fs::read_to_string(d.path().parent().unwrap().parent().unwrap().join("model"))
                        .unwrap_or("None".into())
                ),
                d,
            )
        })
        .collect()
}
