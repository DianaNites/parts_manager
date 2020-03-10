use cursive::{
    align::HAlign,
    traits::Resizable,
    view::{IntoBoxedView, View},
    views::{LinearLayout, Panel, ScrollView, SelectView, TextView},
};

pub fn selection<T: 'static>() -> SelectView<T> {
    SelectView::new().h_align(HAlign::Center)
}

pub fn panel<V: View>(title: &str, v: V) -> Panel<V> {
    Panel::new(v).title(title).title_position(HAlign::Left)
}

/// Panel with a box for other info, containing a view `selecting`.
///
/// Updating information in the info box is not handled by this function
pub fn info_box_panel<V: View, BV: IntoBoxedView + 'static>(
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
