use super::{components::*, get_info_block, Info};
use crate::cli;
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

pub type DiskSelect = SelectView<Info>;
pub type PartSelect = SelectView<Option<Partition>>;
pub type FormatSelect = SelectView<cli::Format>;

fn dump_button(
    root: &mut Cursive,
    gpt: Gpt,
    block_size: BlockSize,
    device_size: Size,
    model: String,
) {
    let mut view: FormatSelect = selection();
    for var in &cli::Format::variants() {
        view.add_item(
            *var,
            cli::Format::from_str(var).expect("Couldn't get variant from itself.."),
        )
    }
    view.set_on_submit(move |root: &mut Cursive, format: &cli::Format| {
        let text = match cli::dump(
            *format,
            cli::PartitionInfo::new(
                &gpt,
                block_size,
                device_size,
                model.clone(),
                Default::default(),
            ),
        ) {
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

pub fn parts(gpt: Gpt, info: &Info) -> impl View {
    let name = &info.name;
    let block_size = info.block_size;
    let device_size = info.disk_size;
    let model = info.model.clone();
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
            dump_button(root, gpt.clone(), block_size, device_size, model.clone());
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

pub fn disks() -> Result<impl View> {
    let mut disks: Vec<Block> = Block::get_connected().context("Couldn't get connected devices")?;
    disks.sort_unstable_by(|a, b| a.name().cmp(b.name()));
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
    let info = vec![DummyView];
    disks_view.set_on_submit(|mut root, info| {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&info.path);
        match file.with_context(|| format!("Couldn't open: `{}`", info.path.display())) {
            Ok(file) => {
                let gpt: Result<Gpt, _> = Gpt::from_reader(file, info.block_size);
                match gpt {
                    Ok(gpt) => {
                        root.add_fullscreen_layer(parts(gpt, info));
                        root.call_on_name("parts", |v: &mut PartSelect| v.set_selection(0))
                            .expect("Missing callback")(&mut root);
                    }
                    Err(e) => {
                        let info = info.clone();
                        let dialog = error(e).button("New GPT", move |mut root| {
                            let gpt: Gpt =
                                Gpt::new(Uuid::new_v4(), info.disk_size, info.block_size);
                            root.pop_layer();
                            root.add_fullscreen_layer(parts(gpt, &info));
                            root.call_on_name("parts", |v: &mut PartSelect| v.set_selection(0))
                                .expect("Missing callback")(&mut root);
                        });
                        root.add_layer(dialog);
                    }
                }
            }
            Err(e) => {
                root.add_layer(error(e));
            }
        }
        //
    });
    let disks = info_box_panel("Disks", disks_view.with_name("disks").full_screen(), info);
    Ok(disks)
}
