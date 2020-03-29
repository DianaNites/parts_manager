use super::components::*;
use crate::{
    actions::{dump, new_gpt, read_gpt_path, Format},
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
    views::{Button, Dialog, DummyView, EditView, LinearLayout, SelectView, TextContent, TextView},
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

fn parts_shared(root: &mut Cursive, info: &Info, quit: ErrAction) {
    err(
        root,
        quit,
        |d| {
            let info = info.clone();
            d.button("New Gpt", move |root| {
                let gpt = new_gpt(None, &info);
                root.pop_layer();
                root.add_fullscreen_layer(parts_impl(gpt, &info));
                setup_views(root);
            })
        },
        |root| {
            let gpt = read_gpt_path(&info)?;
            root.add_fullscreen_layer(parts_impl(gpt, &info));
            setup_views(root);
            //
            Ok(())
        },
    );
}

fn disks_impl() -> Result<impl View> {
    let disks: Vec<Block> = Block::get_connected().context("Couldn't get connected devices")?;
    let mut disks_view: DiskSelect = selection::<Info>();
    for disk in disks {
        let label = format!(
            "Disk {} - {} - Model: {}",
            disk.name(), //
            Byte::from_bytes(disk.size()?.into()).get_appropriate_unit(true),
            disk.model()?.unwrap_or_else(|| "None".into()),
        );
        disks_view.add_item(label, Info::new_block(&disk)?);
    }
    disks_view.set_on_submit(|root, info| {
        parts_shared(root, info, Dismiss);
    });
    let disks = info_box_panel(
        "Disks",
        disks_view.with_name("disks").full_screen(),
        vec![DummyView],
    );
    Ok(disks)
}

/// Helper to setup views due to cursive oddities
fn setup_views(root: &mut Cursive) {
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
fn parts_impl(gpt: Gpt, info: &Info) -> impl View {
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
    let part_name = TextContent::new("");
    let part_start = TextContent::new("");
    let part_size = TextContent::new("");
    let part_uuid = TextContent::new("");
    let part_type = TextContent::new("");
    let info = vec![
        TextView::new_with_content(part_name.clone()),
        TextView::new_with_content(part_start.clone()),
        TextView::new_with_content(part_size.clone()),
        TextView::new_with_content(part_uuid.clone()),
        TextView::new_with_content(part_type.clone()),
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
        //
        part_name.set_content(format!("Name: {}", part.name()));
        part_start.set_content(format!("Start: {}", part.start()));
        part_size.set_content(format!(
            "Size: {}",
            Byte::from_bytes((part.end().0 - part.start().0 + block_size.0).into())
                .get_appropriate_unit(true)
        ));
        part_uuid.set_content(format!("UUID: {}", part.uuid()));
        part_type.set_content(format!("Type: {}", part.partition_type()));
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
pub fn disks(root: &mut Cursive) {
    err(
        root,
        Quit,
        |d| d,
        |root| {
            root.add_fullscreen_layer(disks_impl()?);
            setup_views(root);
            Ok(())
        },
    );
}

pub fn parts(root: &mut Cursive, info: &Info) {
    parts_shared(root, info, Quit);
}
