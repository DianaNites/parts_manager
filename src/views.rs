use super::components::*;
use anyhow::{anyhow, Context, Result};
use byte_unit::Byte;
use cursive::{
    traits::Resizable,
    view::{Nameable, View},
    views::{Dialog, TextView},
    Cursive,
};
use linapi::system::devices::block::Block;
use parts::{types::*, uuid::Uuid, Gpt, Partition};
use std::{fs, path::PathBuf};

pub fn partition_view(
    file: fs::File,
    logical_block_size: u64,
    size: u64,
    name: &str,
) -> Result<impl View> {
    let gpt: Gpt = Gpt::from_reader(file, BlockSize(logical_block_size), Size::from_bytes(size))?;
    let parts = gpt.partitions();
    let mut parts_view = selection();
    for (i, part) in parts.iter().enumerate() {
        let label = format!("Partition {}", i);
        parts_view.add_item(label, *part);
    }
    // TODO: on_select
    let info = vec![
        TextView::empty().with_name("part_name"),
        TextView::empty().with_name("part_uuid"),
    ];
    //
    let parts = info_box_panel(
        &format!("Partitions ({})", name),
        parts_view.with_name("parts").full_screen(),
        info,
        None,
    );
    Ok(parts)
}

pub fn disk_selection() -> Result<impl View> {
    let mut disks: Vec<Block> = Block::get_connected().context("Couldn't get connected devices")?;
    disks.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let mut disks_view = selection();
    for disk in disks {
        let label = format!(
            "Disk {} - {} - Model: {}",
            disk.name(), //
            Byte::from_bytes(disk.size()?.into()).get_appropriate_unit(true),
            disk.model()?.unwrap_or_else(|| "None".into()),
        );
        disks_view.add_item(label, disk);
    }
    let info = vec![
        TextView::empty().with_name("label"),
        TextView::empty().with_name("uuid"),
    ];
    //
    disks_view.set_on_select(|root: &mut Cursive, _dev: &Block| {
        // Unwraps are okay, if not is a bug.
        root.call_on_name("label", |v: &mut TextView| {
            // TODO: label/uuid
            v.set_content("Label: Unknown");
        })
        .unwrap();
        root.call_on_name("uuid", |v: &mut TextView| {
            // TODO: label/uuid
            v.set_content("UUID: Unknown");
        })
        .unwrap();
    });

    disks_view.set_on_submit(|root, dev| {
        fn imp(dev: &Block) -> Result<impl View> {
            let f = dev
                .open()?
                .ok_or_else(|| anyhow!("Device file for `{}` missing", dev.name()))?;
            partition_view(f, dev.logical_block_size()?, dev.size()?, dev.name())
        }
        match imp(dev).with_context(|| {
            format!(
                "Couldn't open device: `{}`\nPath: {}",
                dev.name(),
                dev.dev_path()
                    .unwrap_or_default()
                    .unwrap_or_default()
                    .display()
            )
        }) {
            Ok(v) => {
                root.add_fullscreen_layer(v);
            }
            Err(e) => {
                let dev = dev.clone();
                let dialog = Dialog::info(format!("{:?}", e)).title("Error").button(
                    "Create New Gpt",
                    move |root| {
                        let dev = dev.clone();
                        let dialog = Dialog::new()
                    .content(TextView::new(
                        "Create new GPT Label?\nThis will overwrite any existing content on disk",
                    ))
                    .title("Are you sure?")
                    .button("Yes", move |root| {
                        fn imp(dev: &Block) -> Result<()> {
                            let gpt: Gpt = Gpt::new(Uuid::new_v4());
                            Ok(gpt.to_writer(
                                dev.open()?.unwrap(),
                                dev.logical_block_size()?.into(),
                                Size::from_bytes(dev.size()?),
                            )?)
                        }
                        match imp(&dev) {
                            Ok(_) => (),
                            Err(e) => {
                                let dialog = Dialog::info(format!("{:?}", e)).title("Error");
                                root.add_layer(dialog);
                            }
                        }
                    })
                    .dismiss_button("No");
                        root.add_layer(dialog);
                    },
                );
                root.add_layer(dialog);
            }
        };
    });

    let disks = info_box_panel(
        "Disks",
        disks_view.with_name("disks").full_screen(),
        info,
        None,
    );
    //
    Ok(disks)
}

//

pub struct Data {
    pub path: PathBuf,
    pub name: String,
    pub block_size: BlockSize,
    pub size: Size,
    pub gpt: Option<Gpt>,
}

impl Data {
    pub fn new(dev: Block) -> Result<Self> {
        // let path = dev.path().to_path_buf();
        let path = dev
            .dev_path()?
            .ok_or_else(|| anyhow!("Device file for `{}` missing", dev.name()))?;
        let block_size = BlockSize(dev.logical_block_size()?);
        let size = Size::from_bytes(dev.size()?);
        Self::from_path(path, dev.name().to_owned(), block_size, size)
    }

    pub fn from_path(
        path: PathBuf,
        name: String,
        block_size: BlockSize,
        size: Size,
    ) -> Result<Self> {
        let gpt = {
            let f = fs::File::open(&path);
            match f {
                Ok(f) => Gpt::from_reader(f, block_size, size).ok(),
                _ => None,
            }
        };
        Ok(Self {
            path,
            name,
            block_size,
            size,
            gpt,
        })
    }
}

pub fn part_view(data: &Data) -> Result<impl View> {
    let gpt: &Gpt = data.gpt.as_ref().unwrap();
    let name = &data.name;
    //
    let parts = gpt.partitions();
    let mut parts_view = selection();
    for (i, part) in parts.iter().enumerate() {
        let label = format!("Partition {}", i);
        dbg!(&part);
        parts_view.add_item(label, *part);
    }
    let info = vec![
        TextView::empty().with_name("part_name"),
        TextView::empty().with_name("part_uuid"),
        TextView::empty().with_name("part_type"),
    ];
    parts_view.set_on_select(|root: &mut Cursive, part: &Partition| {
        // Unwraps are okay, if not is a bug.
        root.call_on_name("part_name", |v: &mut TextView| {
            v.set_content(format!("Name: {}", part.name()));
        })
        .unwrap();
        root.call_on_name("part_uuid", |v: &mut TextView| {
            v.set_content(format!("UUID: {}", part.uuid()));
        })
        .unwrap();
        root.call_on_name("part_type", |v: &mut TextView| {
            v.set_content(format!("Type: {}", part.partition_type()));
        })
        .unwrap();
    });
    //
    let parts = info_box_panel(
        &format!("Partitions ({})", name),
        parts_view.with_name("parts").full_screen(),
        info,
        None,
    );
    Ok(parts)
}

pub fn disks() -> Result<impl View> {
    let mut disks: Vec<Block> = Block::get_connected().context("Couldn't get connected devices")?;
    disks.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let mut disks_view = selection();
    for disk in disks {
        let label = format!(
            "Disk {} - {} - Model: {}",
            disk.name(), //
            Byte::from_bytes(disk.size()?.into()).get_appropriate_unit(true),
            disk.model()?.unwrap_or_else(|| "None".into()),
        );
        disks_view.add_item(label, Data::new(disk)?);
    }
    let info = vec![TextView::empty().with_name("uuid")];
    disks_view.set_on_select(|root: &mut Cursive, dev: &Data| {
        // Unwraps are okay, if not is a bug.
        root.call_on_name("uuid", |v: &mut TextView| {
            if let Some(gpt) = &dev.gpt {
                v.set_content(format!("UUID: {}", gpt.uuid()));
            } else {
                v.set_content(format!("UUID: {}", "Unknown"));
            }
        })
        .unwrap();
    });
    disks_view.set_on_submit(|root, data| {
        //
        root.add_fullscreen_layer(part_view(data).unwrap())
    });
    let disks = info_box_panel(
        "Disks",
        disks_view.with_name("disks").full_screen(),
        info,
        None,
    );
    Ok(disks)
}
