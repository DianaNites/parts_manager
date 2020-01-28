use crate::util::{get_disks, Disk};
use cursive::{
    align::{HAlign, VAlign},
    event::Key,
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    traits::Boxable,
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

fn parts_edit(root: &mut Cursive, _disk: &Disk) {
    root.pop_layer();
    //
    let mut parts = SelectView::new();
    parts.add_item("Dummy", ());
    parts.add_item("Dummy", ());
    //
    let mut devices = LinearLayout::vertical();
    devices.add_child(TextView::new("Device").h_align(HAlign::Left));
    devices.add_child(DummyView);
    // devices.add_child(TextView::new("Dummy Part 1").h_align(HAlign::Left));
    // devices.add_child(TextView::new("Dummy Part 2").h_align(HAlign::Left));
    // devices.add_child(TextView::new("Dummy Part 3").h_align(HAlign::Left));
    devices.add_child(parts);

    let mut starts = LinearLayout::vertical();
    starts.add_child(TextView::new("Start").h_align(HAlign::Right));
    starts.add_child(DummyView);
    starts.add_child(TextView::new("1").h_align(HAlign::Right));
    starts.add_child(TextView::new("2").h_align(HAlign::Right));
    starts.add_child(TextView::new("3").h_align(HAlign::Right));
    //
    root.add_fullscreen_layer(
        Panel::new(
            LinearLayout::vertical()
                .child(
                    LinearLayout::horizontal()
                        .child(devices)
                        .child(DummyView.full_width())
                        .child(starts)
                        // .child(TextView::new("Device").h_align(HAlign::Left).full_width())
                        // .child(TextView::new("Start").h_align(HAlign::Center).full_width())
                        .child(TextView::new("End").h_align(HAlign::Center).full_width())
                        .child(TextView::new("Size").h_align(HAlign::Center).full_width())
                        .child(TextView::new("Type").h_align(HAlign::Center).full_width()),
                )
                .child(DummyView), // .child(parts),
        )
        .full_screen(),
    );
}

fn disk_selection(root: &mut Cursive) {
    let mut disks = SelectView::new().h_align(HAlign::Center);
    disks.add_all(get_disks());
    disks.set_on_submit(parts_edit);
    //
    let select = Panel::new(
        LinearLayout::vertical()
            .child(TextView::new(format!(
                "Part {}, {}",
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
