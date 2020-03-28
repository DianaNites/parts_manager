use super::components::*;
use crate::{
    actions::{dump, Format},
    get_info_block,
    Info,
};
use anyhow::{Context, Result};
use byte_unit::Byte;
use cursive::{
    align::HAlign,
    theme::{BaseColor, Color, ColorStyle, ColorType, Effect, Style},
    traits::Resizable,
    utils::markup::StyledString,
    view::{Nameable, View},
    views::{Button, Dialog, DummyView, EditView, LinearLayout, SelectView, TextView},
    Cursive,
};
use linapi::system::devices::block::Block;
use parts::{types::*, uuid::Uuid, Gpt, Partition, PartitionBuilder, PartitionType};
use std::{fs, path::Path, str::FromStr};

type DiskSelect = SelectView<Info>;
type PartSelect = SelectView<Option<Partition>>;
type FormatSelect = SelectView<Format>;

/// Dump the GPT Partition to a file
fn dump_button(root: &mut Cursive, gpt: Gpt, info: Info) {
    let mut view: FormatSelect = selection();
    for var in &Format::variants() {
        view.add_item(
            *var,
            Format::from_str(var).expect("Couldn't get variant from itself.."),
        )
    }
    view.set_on_submit(move |root: &mut Cursive, format: &Format| {
        let text = match dump(&gpt, *format, &info) {
            Ok(t) => {
                root.pop_layer();
                t
            }
            Err(e) => {
                root.add_layer(error(e).button("Cancel", |root| {
                    root.pop_layer();
                    root.pop_layer();
                }));
                return;
            }
        };
        let view = EditView::new()
            .on_submit(move |root, s| {
                match fs::write(Path::new(s), &text) {
                    Ok(_) => {
                        root.pop_layer();
                    }
                    Err(e) => root.add_layer(error(e)),
                }
                //
            })
            .min_width(20);
        let view = Dialog::around(view)
            .dismiss_button("Cancel")
            .title("Dump File")
            .title_position(HAlign::Left);
        root.add_layer(view);
    });
    let title = "Select format";
    root.add_layer(panel(title, view).min_width(title.len() + 6));
}

fn select_disk(info: &Info) -> Result<Gpt> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&info.path)
        .with_context(|| format!("Couldn't open: `{}`", info.path.display()))?;
    let gpt: Gpt = Gpt::from_reader(file, info.block_size)?;
    Ok(gpt)
}

fn new_gpt(info: &Info) -> Gpt {
    Gpt::new(Uuid::new_v4(), info.disk_size, info.block_size)
}

/// Helper to setup views due to cursive oddities
pub fn setup_views(root: &mut Cursive) {
    if root.user_data::<Partition>().is_none() {
        // Required for `parts`, it'll start unset and crash if no partitions
        root.set_user_data(None::<Partition>);
    }

    // Make sure the selection callback is run so the info box is populated.
    //
    // If theres a current selection, like when running this for `parts`, don't
    // change it.
    //
    // This may be None, when the user provides a path.
    if let Some(cb) = root.call_on_name("disks", |v: &mut DiskSelect| {
        v.set_selection(v.selected_id().unwrap_or(0))
    }) {
        cb(root)
    }

    // Make sure the parts callback is run. This won't always exist, for example
    // when setting up `disks`.
    //
    // `disks` will call this itself.
    if let Some(cb) = root.call_on_name("parts", |v: &mut PartSelect| v.set_selection(0)) {
        cb(root);
    }
}

/// Partition editing view.
pub fn parts(gpt: Gpt, info: &Info) -> impl View {
    let name = &info.name;
    let block_size = info.block_size;
    let new_info = info.clone();
    let remaining = gpt.remaining();
    let parts = gpt.partitions();
    let mut parts_view: PartSelect = selection();
    for (i, part) in parts.iter().enumerate() {
        let label = format!("Partition {}", i + 1);
        parts_view.add_item(label, Some(*part));
    }
    parts_view.add_item(
        StyledString::styled(
            "Free Space",
            Style {
                effects: Effect::Bold.into(),
                color: Some(ColorStyle {
                    front: ColorType::Color(Color::Dark(BaseColor::Green)),
                    // FIXME: https://github.com/gyscos/cursive/issues/284
                    ..ColorStyle::primary()
                }),
            },
        ),
        None,
    );
    let info = vec![
        TextView::empty().with_name("part_name"),
        TextView::empty().with_name("part_start"),
        TextView::empty().with_name("part_size"),
        TextView::empty().with_name("part_uuid"),
        TextView::empty().with_name("part_type"),
    ];
    parts_view.set_on_select(move |root: &mut Cursive, part: &Option<Partition>| {
        let part_ = root
            .with_user_data(|last: &mut Option<Partition>| {
                part.unwrap_or(
                    PartitionBuilder::new(Uuid::nil())
                        .name("None")
                        .start(
                            last.map(|p| Offset(p.end().0 + 1))
                                .unwrap_or_else(|| Size::from_mib(1).into()),
                        )
                        .size(remaining)
                        .partition_type(PartitionType::Unused)
                        .finish(block_size),
                )
            })
            .unwrap_or_else(|| part.expect("What the fuck"));
        let part = part_;
        root.set_user_data(Some(part));
        // Unwraps are okay, if not is a bug.
        root.call_on_name("part_name", |v: &mut TextView| {
            v.set_content(format!("Name: {}", part.name()));
        })
        .expect("Missing callback");
        root.call_on_name("part_start", |v: &mut TextView| {
            v.set_content(format!("Start: {}", part.start()));
        })
        .expect("Missing callback");
        root.call_on_name("part_size", |v: &mut TextView| {
            v.set_content(format!(
                "Size: {}",
                Byte::from_bytes((part.end().0 - part.start().0 + block_size.0).into())
                    .get_appropriate_unit(true)
            ));
        })
        .expect("Missing callback");
        root.call_on_name("part_uuid", |v: &mut TextView| {
            v.set_content(format!("UUID: {}", part.uuid()));
        })
        .expect("Missing callback");
        root.call_on_name("part_type", |v: &mut TextView| {
            v.set_content(format!("Type: {}", part.partition_type()));
        })
        .expect("Missing callback");
    });
    //
    let mut buttons = LinearLayout::horizontal()
        .child(DummyView.full_width())
        .child(Button::new("Dump", move |root| {
            dump_button(root, gpt.clone(), new_info.clone());
        }))
        .child(DummyView)
        .child(Button::new("Test 2", |_| ()))
        .child(DummyView.full_width());
    buttons
        .set_focus_index(1)
        .expect("First button didn't accept focus");
    let buttons = focused_view(buttons.with_name("buttons"));
    //
    horizontal_forward::<LinearLayout, _>(
        info_box_panel_footer(
            &format!("Partitions ({})", name),
            parts_view.with_name("parts").full_screen(),
            info,
            buttons,
        ),
        "buttons",
    )
}

/// Returns a view that allows the user to select a disk,
/// then calling [`parts`].
pub fn disks() -> Result<impl View> {
    let disks: Vec<Block> = Block::get_connected().context("Couldn't get connected devices")?;
    let mut disks_view: DiskSelect = selection::<Info>();
    for disk in disks {
        let label = format!(
            "Disk {} - {} - Model: {}",
            disk.name(), //
            Byte::from_bytes(disk.size()?.into()).get_appropriate_unit(true),
            disk.model()?.unwrap_or_else(|| "None".into()),
        );
        disks_view.add_item(label, get_info_block(&disk)?);
    }
    disks_view.set_on_submit(|root, info| {
        let gpt = select_disk(info);
        match gpt {
            Ok(gpt) => {
                root.add_fullscreen_layer(parts(gpt, info));
                setup_views(root);
            }
            Err(e) => {
                let info = info.clone();
                let dialog = error(e).button("New GPT", move |root| {
                    let gpt = new_gpt(&info);
                    root.pop_layer();
                    root.add_fullscreen_layer(parts(gpt, &info));
                    setup_views(root);
                });
                root.add_layer(dialog);
            }
        }
    });
    let disks = info_box_panel(
        "Disks",
        disks_view.with_name("disks").full_screen(),
        vec![DummyView],
    );
    Ok(disks)
}
