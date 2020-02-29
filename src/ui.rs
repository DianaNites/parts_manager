use crate::util::get_disks;
use anyhow::{anyhow, Context, Result};
use cursive::{
    align::{HAlign, VAlign},
    event::Key,
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    traits::Resizable,
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

fn parts_edit(mut root: &mut Cursive, disk: &Block) -> Result<()> {
    root.pop_layer();
    let mut parts = SelectView::new().h_align(HAlign::Center);
    let mut p: Vec<Partition> = error(root, || Ok(disk.partitions()?))?;
    for p in p {
        let start = error(root, || Ok(p.start()?))?;
        let size = error(root, || Ok(p.size()?))?;
        let s = format!(
            "{} ({}) {} {} {} {}",
            error(root, || Ok(p.number()?))?,
            p.name(),
            start,
            start + size,
            size,
            "Unknown"
        );
        parts.add_item(s, p);
    }
    parts.sort_by_label();
    //
    let select = Panel::new(
        LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("Partition (device)"))
                    .child(DummyView)
                    .child(TextView::new("Start (sectors)"))
                    .child(DummyView)
                    .child(TextView::new("End (sectors)"))
                    .child(DummyView)
                    .child(TextView::new("Size"))
                    .child(DummyView)
                    .child(TextView::new("Type")),
            )
            .child(parts
                // LinearLayout::vertical()
                //     .child(TextView::new("1 (sda1)"))
                //     .child(TextView::new("1024"))
                //     .child(TextView::new("2048"))
                //     .child(TextView::new("1 MiB"))
                //     .child(TextView::new("Unknown")),
            ),
    )
    .full_screen();
    //
    root.add_fullscreen_layer(select);
    Ok(())
}

fn disk_selection(mut root: &mut Cursive) -> Result<()> {
    let mut disks = SelectView::new().h_align(HAlign::Center);
    let mut d = get_disks().context("Couldn't get disks")?;
    d.sort_unstable_by(|a, b| a.1.name().cmp(b.1.name()));
    disks.add_all(d);
    disks.set_on_submit(parts_edit);
    //
    let select = Panel::new(
        LinearLayout::vertical()
            .child(TextView::new(format!(
                "Parts {}, {}",
                std::env!("CARGO_PKG_VERSION"),
                std::env!("CARGO_PKG_DESCRIPTION")
            )))
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
