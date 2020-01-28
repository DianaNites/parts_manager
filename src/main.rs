use anyhow::Result;

mod ui;
mod util;
use ui::create_ui;

fn main() -> Result<()> {
    create_ui();
    Ok(())
}
