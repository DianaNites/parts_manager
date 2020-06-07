//! Code for the interactive TUI interface
use crate::Info;
use anyhow::Result;
use cursive::{
    event::Event,
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    Cursive,
};

pub mod components;
pub mod views;

use views::{disks, parts};

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

/// Handle the TUI interface.
///
/// This function doesn't return until the user exits.
pub fn handle_tui(info: Option<Info>) -> Result<()> {
    let mut root = cursive::default();
    // Theme
    let theme = theme(&mut root);
    root.set_theme(theme);

    // User entry point
    if info.is_none() {
        disks(&mut root);
    } else if let Some(info) = info {
        parts(&mut root, &info);
    }

    // Global hotkeys
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback(Event::CtrlChar('d'), |s| s.toggle_debug_console());
    root.add_global_callback('h', |_| todo!("Help menu"));

    root.run();
    Ok(())
}
