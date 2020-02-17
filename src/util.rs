use byte_unit::Byte;
use linapi::{
    devices::BlockDevice,
    types::{BlockDevice as _, Device as _},
};
use std::fs;

pub fn get_disks() -> Vec<(String, BlockDevice)> {
    BlockDevice::get_connected()
        .into_iter()
        .map(|d| {
            (
                format!(
                    "Disk {} - {} - Model: {}",
                    d.kernel_name(),
                    Byte::from_bytes(d.size().into()).get_appropriate_unit(true),
                    fs::read_to_string(
                        d.device_path()
                            .parent()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join("model")
                    )
                    .unwrap_or("None".into())
                ),
                d,
            )
        })
        .collect()
}
