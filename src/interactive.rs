//! Code for the interactive TUI interface
use anyhow::Result;
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    Cursive,
};

pub mod components;
pub mod views;

/// Set a better default theme. Assumes dark terminal. Untested on light.
// TODO: Runtime Theme loading?
fn theme(root: &mut Cursive) -> Theme {
    let mut theme = root.current_theme().clone();
    theme.palette[Background] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = Dark(White);
    theme.palette[Tertiary] = Dark(White);
    theme
}

/// Handle the TUI interface
pub fn handle_tui() -> Result<()> {
    let mut root = Cursive::default();
    // Theme
    let theme = theme(&mut root);
    root.set_theme(theme);
    //
    Ok(())
}
