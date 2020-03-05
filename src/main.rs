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
    types::{BlockSize, ByteSize},
    Gpt,
};
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

#[derive(StructOpt)]
#[structopt(global_setting(AppSettings::ColoredHelp))]
struct Args {
    /// Path to device or file to partition.
    ///
    /// If not provided, you can choose between connected block devices.
    device: Option<PathBuf>,
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

fn partition_view(root: &mut Cursive, dev: &Block) {
    fn imp(dev: &Block) -> Result<impl View> {
        let f = dev
            .open()?
            .ok_or_else(|| anyhow!("Device file for `{}` missing", dev.name()))?;
        let gpt = Gpt::from_reader(
            f,
            BlockSize(dev.logical_block_size()?),
            ByteSize::from_bytes(dev.size()?),
        )?;
        // FIXME: Terrible hack.
        let gpt = Box::leak(Box::new(gpt));
        let parts = gpt.partitions();
        let mut parts_view = selection();
        for (i, part) in parts.iter().enumerate() {
            let label = format!("Partition {}", i);
            parts_view.add_item(label, part);
        }
        // TODO: on_select
        let info = vec![
            TextView::empty().with_name("part_name"),
            TextView::empty().with_name("part_uuid"),
        ];
        //
        let parts = info_box_panel(
            &format!("Partitions ({})", dev.name()),
            parts_view.with_name("parts").full_screen(),
            info,
            None,
        );
        Ok(parts)
    };
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
            let dialog = Dialog::info(format!("{:?}", e)).title("Error");
            root.add_layer(dialog);
        }
    };
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

    disks_view.set_on_submit(partition_view);

    let disks = info_box_panel(
        "Disks",
        disks_view.with_name("disks").full_screen(),
        info,
        None,
    );
    //
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
    if args.device.is_none() {
        root.add_fullscreen_layer(disk_selection()?);
        // Disk Info box will start empty, make sure callback is called and it's set.
        root.call_on_name("disks", |v: &mut SelectView<Block>| v.set_selection(0))
            .unwrap()(&mut root);
    } else {
        // partition_view(&mut root);
    }

    // Global hotkeys
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback('h', |_| todo!("Help menu"));
    //
    root.run();
    //
    Ok(())
}
