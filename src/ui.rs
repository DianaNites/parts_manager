use crate::util::get_disks;
use cursive::{
    align::{HAlign, VAlign},
    event::Key,
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    traits::Boxable,
    view::{Finder, Nameable, Selector, View},
    views::{DummyView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};

fn theme(root: &Cursive) -> Theme {
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    theme
}

fn selection_screen(title: &str, list: impl View, name: &str) -> impl View {
    Panel::new(
        LinearLayout::vertical()
            .child(TextView::new(format!(
                "Part {}, {}",
                std::env!("CARGO_PKG_VERSION"),
                std::env!("CARGO_PKG_DESCRIPTION")
            )))
            .child(TextView::new(title).h_align(HAlign::Center))
            .child(DummyView)
            .child(list)
            .with_name(name),
    )
    .full_screen()
}

fn parts_edit(_root: &mut Cursive) -> impl View {
    let mut parts = SelectView::new();
    parts.add_item("Dummy            a", ());
    parts.add_item("Dummy", ());
    parts
}

fn disk_selection(root: &mut Cursive) {
    let mut disks = SelectView::new().h_align(HAlign::Center);
    disks.add_all(get_disks());
    disks.set_on_submit(|root, _| {
        root.pop_layer();
        //
        let parts = parts_edit(root);
        //
        root.add_fullscreen_layer(selection_screen("Select a Partition", parts, "Part List"))
    });
    //
    let mut select = selection_screen("Select a Disk", disks, "Disk List");
    select
        .call_on(&Selector::Name("Disk List"), |list: &mut LinearLayout| {
            list.add_child(DummyView);
            list.add_child(
                TextView::new("If Disk Capacity is incorrect DO NOT continue")
                    .v_align(VAlign::Bottom)
                    .full_screen(),
            )
        })
        .unwrap();

    root.add_fullscreen_layer(select)
}

/// Creates the Cursive UI and runs the event loop.
///
/// This does not return unless the app has been quit.
pub fn create_ui() {
    let mut root = Cursive::default();
    root.set_theme(theme(&root));
    //
    disk_selection(&mut root);
    //
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback(Key::Esc, |s| s.select_menubar());
    //
    root.run();
}
