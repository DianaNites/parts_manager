use anyhow::Error;
use cursive::{
    align::HAlign,
    traits::Resizable,
    view::{IntoBoxedView, View},
    views::{Canvas, Dialog, LinearLayout, Panel, ScrollView, SelectView},
};

pub fn selection<T: 'static>() -> SelectView<T> {
    SelectView::new().h_align(HAlign::Center)
}

pub fn panel<V: View>(title: &str, v: V) -> Panel<V> {
    Panel::new(v).title(title).title_position(HAlign::Left)
}

fn info_box<V: View, BV: IntoBoxedView + 'static>(selecting: V, info: Vec<BV>) -> LinearLayout {
    let mut info_box = LinearLayout::vertical();
    for view in info {
        info_box.add_child(view);
    }
    let info_box = panel("Info", info_box).full_width();
    //
    LinearLayout::vertical()
        .child(ScrollView::new(selecting))
        .child(info_box.full_width())
}

/// Panel with a box for other info, containing a view `selecting`.
///
/// Updating information in the info box is not handled by this function
pub fn info_box_panel<V: View, BV: IntoBoxedView + 'static>(
    title: &str,
    selecting: V,
    info: Vec<BV>,
) -> Panel<impl View> {
    panel(title, info_box(selecting, info).full_screen())
}

pub fn info_box_panel_footer<V: View, BV: IntoBoxedView + 'static>(
    title: &str,
    selecting: V,
    info: Vec<BV>,
    footer: impl View,
) -> Panel<impl View> {
    panel(title, info_box(selecting, info).child(footer).full_screen())
}

pub fn error<E: Into<Error>>(e: E) -> Dialog {
    let e = e.into();
    Dialog::info(format!("{:?}", e)).title("Error")
}

pub fn error_quit<E: Into<Error>>(e: E) -> Dialog {
    let e = e.into();
    Dialog::text(format!("{:?}", e))
        .title("Error")
        .button("Ok", |root| root.quit())
}

/// A view which is always in focus, and which never takes focus from elsewhere.
pub fn focused_view<V: View>(view: V) -> Canvas<V> {
    Canvas::wrap(view)
        .with_take_focus(|_, _| false)
        .with_draw(|s, p| {
            let mut p = p.clone();
            p.focused = true;
            s.draw(&p)
        })
}
