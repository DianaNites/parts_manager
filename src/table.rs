use cursive::{
    direction::Direction,
    event::{Event, EventResult, Key},
    theme::ColorStyle,
    view::{Boxable, Resizable, View},
    views::{DummyView, LinearLayout, TextView},
    Printer,
};

pub struct TableView<T> {
    columns: T,
    rows: Vec<T>,
    enabled: bool,
}

impl<T> TableView<T> {
    pub fn new(columns: T, rows: Vec<T>) -> Self {
        TableView {
            columns,
            rows,
            enabled: true,
        }
    }
}

type TableViewStr = &'static &'static str;

impl<T: 'static, I> View for TableView<T>
where
    I: Iterator<Item = TableViewStr> + Clone,
    T: IntoIterator<Item = TableViewStr, IntoIter = I> + Copy + Clone,
{
    fn draw(&self, printer: &Printer) {
        let mut header = LinearLayout::horizontal();
        for (index, col) in self.columns.into_iter().enumerate() {
            let mut child = LinearLayout::vertical();
            child.add_child(TextView::new(*col));
            for rows in &self.rows {
                child.add_child(TextView::new(*rows.into_iter().nth(index).unwrap()));
            }
            //
            header.add_child(child);
            header.add_child(DummyView.full_width());
        }
        let mut header = header.full_width();
        header.layout(printer.size);
        header.draw(printer);
        printer.with_color(ColorStyle::highlight(), |p| header.draw(p));
        //
        // let iter = self.columns.into_iter();
        // let count = iter.clone().count();
        // let mut offset = 0;
        // let count = self.columns.len();
        // for (index, col) in self.columns.into_iter().enumerate() {
        //     let printer = &printer.offset((offset, 0)).focused(true);
        //     //
        //     printer.with_color(ColorStyle::primary(), |printer| {
        //         let s = col;
        //         printer.print((0, 0), &s);
        //     });
        //     //
        //     if index < (count - 1) {
        //         printer.print((col.len() + 1, 0), "|");
        //     }
        //     offset += col.len() + 3;
        // }
        // printer.print_hline((0, 1), offset - 1, "-");

        // let y = printer.output_size.y / 4;
        // let y = y.max(3);
        // printer.print_box((0, 0), (printer.output_size.x, y), false);
        // let col = printer.offset((1, y / 2)).focused(true);
        // col.print((0, 0), "TEST");
        // col.print_vline((4, 0), y / 2, "|")
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Up) => {
                //
                EventResult::Consumed(None)
            }
            Event::Key(Key::Down) => {
                //
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored,
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }
}
