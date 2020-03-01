use crate::util::get_disks;
use anyhow::{anyhow, Context, Result};
use byte_unit::Byte;
use cursive::{
    align::{HAlign, VAlign},
    event::Key,
    theme::{BaseColor::*, Color::*, Effect, PaletteColor::*, Theme},
    traits::Resizable,
    view::Nameable,
    views::{Dialog, DummyView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};
use linapi::system::devices::block::{Block, Partition};

fn theme(root: &Cursive) -> Theme {
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    theme
}

/// Run a closure, if an error occurs display a popup.
///
/// The user has two choices, Cancel and Quit.
///
/// Cancel should bring them to the previous view, if any.
///
/// Quit should quit the application.
fn error<T, F: FnMut() -> Result<T>>(root: &mut Cursive, mut func: F) -> Result<T> {
    let err: Result<T> = func();
    match err {
        Ok(t) => Ok(t),
        Err(e) => {
            root.add_fullscreen_layer(Dialog::info(e.to_string()));
            //
            // root.pop_layer();
            Err(anyhow!("Couldn't retry"))
        }
    }
}

fn parts_edit(root: &mut Cursive, disk: &Block) -> Result<()> {
    root.pop_layer();
    let mut parts = SelectView::new().h_align(HAlign::Center);
    let p: Vec<Partition> = error(root, || Ok(disk.partitions()?))?;
    for p in p {
        let s = format!(
            "Partition {} (/dev/{})",
            error(root, || Ok(p.number()?))?,
            p.name(),
        );
        parts.add_item(s, p);
    }
    parts.sort_by_label();
    parts.set_on_select(|root: &mut Cursive, part: &Partition| {
        root.call_on_name("start", |v: &mut TextView| {
            v.set_content(format!("Start: {} bytes", part.start().unwrap_or(0)));
        });
        root.call_on_name("end", |v: &mut TextView| {
            v.set_content(format!(
                "End: {} bytes",
                part.start().unwrap_or(0) + part.size().unwrap_or(0)
            ));
        });
        root.call_on_name("size", |v: &mut TextView| {
            v.set_content(format!(
                "Size: {}",
                Byte::from_bytes(part.size().unwrap_or(0).into()).get_appropriate_unit(true)
            ));
        });
    });
    let info_box = Panel::new(
        LinearLayout::vertical() //
            .child(TextView::empty().with_name("start"))
            .child(TextView::empty().with_name("end"))
            .child(TextView::empty().with_name("size")),
    )
    .full_screen();
    //
    let select = Panel::new(
        LinearLayout::vertical()
            .child(parts.with_name("parts"))
            .child(info_box),
    )
    .title(format!("Partition Selection (Disk /dev/{})", disk.name()))
    .title_position(HAlign::Left)
    .full_screen();
    //
    root.add_fullscreen_layer(select);
    // Unwrap is okay, failure indicates a bug
    root.call_on_name("parts", |v: &mut SelectView<Partition>| v.set_selection(0))
        .unwrap()(root);
    //
    Ok(())
}

fn disk_selection(root: &mut Cursive) -> Result<()> {
    let mut disks = SelectView::new().h_align(HAlign::Center);
    let mut d = get_disks().context("Couldn't get disks")?;
    d.sort_unstable_by(|a, b| a.1.name().cmp(b.1.name()));
    disks.add_all(d);
    disks.set_on_submit(parts_edit);
    //
    let select = Panel::new(
        LinearLayout::vertical()
            .child(TextView::new("Select A Disk").h_align(HAlign::Center))
            .child(DummyView)
            .child(disks)
            .child(DummyView)
            .child(
                TextView::new("If Disk Capacity is incorrect DO NOT continue")
                    .v_align(VAlign::Bottom)
                    .full_screen(),
            ),
    )
    .title(format!(
        "Parts {}, {}.",
        std::env!("CARGO_PKG_VERSION"),
        std::env!("CARGO_PKG_DESCRIPTION")
    ))
    .title_position(HAlign::Left)
    .full_screen();

    root.add_fullscreen_layer(select);
    //
    Ok(())
}

/// Creates the Cursive UI and runs the event loop.
///
/// This does not return unless the app has been quit.
pub fn create_ui() -> Result<()> {
    let mut root = Cursive::default();
    root.set_theme(theme(&root));
    //
    disk_selection(&mut root)?;
    //
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback(Key::Esc, |s| s.select_menubar());
    //
    root.run();
    Ok(())
}
