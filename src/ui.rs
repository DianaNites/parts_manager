use crate::util::get_disks;
use cursive::{
    align::{HAlign, VAlign},
    event::Key,
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    traits::Resizable,
    views::{DummyView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};
use linapi::system::devices::block::Block;

fn theme(root: &Cursive) -> Theme {
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    theme
}

fn parts_edit(root: &mut Cursive, _disk: &Block) {
    root.pop_layer();

    //
    let columns: &[&str; 5] = &["Device", "Start", "End", "Size", "Type"];
    let rows = vec![&["One", "1", "2", "3", "Test"]];
    // root.add_fullscreen_layer(
    //     Panel::new(TableView::new(columns,
    // rows).full_screen()).full_screen(), );
}

fn disk_selection(root: &mut Cursive) {
    let mut disks = SelectView::new().h_align(HAlign::Center);
    let mut d = get_disks();
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
