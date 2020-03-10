use anyhow::{anyhow, Context, Result};
use byte_unit::Byte;
use cursive::{
    align::HAlign,
    theme::{BaseColor::*, Color::*, PaletteColor::*},
    traits::Resizable,
    view::{IntoBoxedView, Nameable, View},
    views::{Dialog, LinearLayout, Panel, ScrollView, SelectView, TextView},
    Cursive,
};
use linapi::system::devices::block::Block;
use parts::{
    types::{BlockSize, ByteSize, LogicalBlockAddress},
    Gpt,
    Partition,
};
use std::{fs, path::PathBuf};
use structopt::{
    clap::{arg_enum, AppSettings},
    StructOpt,
};

#[derive(Debug, StructOpt)]
#[structopt(global_setting(AppSettings::ColoredHelp))]
struct Args {
    /// Path to device or file to partition.
    ///
    /// If not provided, you can choose between connected block devices.
    device: Option<PathBuf>,

    /// Logical Block Size to use. Ignored when `device` is not specified.
    #[structopt(long, default_value("512"))]
    block: u64,
}

fn selection<T: 'static>() -> SelectView<T> {
    SelectView::new().h_align(HAlign::Center)
}

fn panel<V: View>(title: &str, v: V) -> Panel<V> {
    Panel::new(v).title(title).title_position(HAlign::Left)
}

/// Panel with a box for other info, containing a view `selecting`.
///
/// Updating information in the info box is not handled by this function
fn info_box_panel<V: View, BV: IntoBoxedView + 'static>(
    title: &str,
    selecting: V,
    info: Vec<BV>,
    footer: Option<&str>,
) -> Panel<impl View> {
    let mut info_box = LinearLayout::vertical();
    for view in info {
        info_box.add_child(view);
    }
    let info_box = panel("Info", info_box).full_width();
    //
    let mut l = LinearLayout::vertical()
        .child(ScrollView::new(selecting))
        .child(info_box.full_screen());
    if let Some(s) = footer {
        l.add_child(TextView::new(s).h_align(HAlign::Right));
    }
    //
    panel(title, l.full_screen())
}

fn partition_view(
    file: fs::File,
    logical_block_size: u64,
    size: u64,
    name: &str,
) -> Result<impl View> {
    let gpt = Gpt::from_reader(
        file,
        BlockSize(logical_block_size),
        ByteSize::from_bytes(size),
    )?;
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

fn disk_selection() -> Result<impl View> {
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
                            let gpt = Gpt::new();
                            Ok(gpt.to_writer(
                                dev.open()?.unwrap(),
                                dev.logical_block_size()?.into(),
                                ByteSize::from_bytes(dev.size()?),
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

struct Data {
    path: PathBuf,
    name: String,
    block_size: BlockSize,
    size: ByteSize,
    gpt: Option<Gpt>,
}

impl Data {
    fn new(dev: Block) -> Result<Self> {
        // let path = dev.path().to_path_buf();
        let path = dev
            .dev_path()?
            .ok_or_else(|| anyhow!("Device file for `{}` missing", dev.name()))?;
        let block_size = BlockSize(dev.logical_block_size()?);
        let size = ByteSize::from_bytes(dev.size()?);
        Self::from_path(path, dev.name().to_owned(), block_size, size)
    }

    fn from_path(
        path: PathBuf,
        name: String,
        block_size: BlockSize,
        size: ByteSize,
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

fn part_view(data: &Data) -> Result<impl View> {
    let gpt = data.gpt.unwrap();
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

fn disks() -> Result<impl View> {
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
            if let Some(gpt) = dev.gpt {
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

fn main() -> Result<()> {
    let args: Args = Args::from_args();
    //
    let mut root = Cursive::default();
    // Theme
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    root.set_theme(theme);

    // User Entry point
    match args.device {
        Some(path) => {
            let block_size = BlockSize(args.block);
            let disk_size = ByteSize::from_bytes(fs::metadata(&path).unwrap().len());
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            root.add_fullscreen_layer(part_view(&Data::from_path(
                path, name, block_size, disk_size,
            )?)?);
            // Info box will start empty, make sure callback is called and it's set.
            root.call_on_name("parts", |v: &mut SelectView<Partition>| v.set_selection(0))
                .unwrap()(&mut root);
        }
        None => {
            root.add_fullscreen_layer(disks()?);
            // Disk Info box will start empty, make sure callback is called and it's set.
            root.call_on_name("disks", |v: &mut SelectView<Data>| v.set_selection(0))
                .unwrap()(&mut root);
        }
    }
    // if args.device.is_none() {
    //     root.add_fullscreen_layer(disk_selection()?);
    //     // Disk Info box will start empty, make sure callback is called and it's
    // set.     root.call_on_name("disks", |v: &mut SelectView<Block>|
    // v.set_selection(0))         .unwrap()(&mut root);
    // } else {
    //     // partition_view(&mut root);
    // }

    // Global hotkeys
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback('h', |_| todo!("Help menu"));
    //
    root.run();
    //
    Ok(())
}
