//! Code for the interactive TUI interface
use crate::Info;
use anyhow::Result;
use cursive::{
    theme::{BaseColor::*, Color::*, PaletteColor::*, Theme},
    Cursive,
};
use parts::{uuid::Uuid, Gpt};
use std::fs;

pub mod components;
pub mod views;

use components::error_quit;
use views::{disks, parts, setup_views};

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
    let mut root = Cursive::default();
    // Theme
    let theme = theme(&mut root);
    root.set_theme(theme);

    // User entry point
    if info.is_none() {
        disks(&mut root);
    } else if let Some(info) = info {
        let gpt: Result<Gpt, _> = Gpt::from_reader(
            fs::OpenOptions::new()
                .write(true)
                .read(true)
                .open(&info.path)?,
            info.block_size,
        );
        match gpt {
            Ok(gpt) => {
                root.add_fullscreen_layer(parts(gpt, &info));
                setup_views(&mut root);
            }
            Err(e) => {
                root.add_layer(error_quit(e).button("New Gpt", move |mut root| {
                    let gpt: Gpt = Gpt::new(Uuid::new_v4(), info.disk_size, info.block_size);
                    root.pop_layer();
                    root.add_fullscreen_layer(parts(gpt, &info));
                    setup_views(&mut root);
                }));
            }
        };
    }

    // Global hotkeys
    root.add_global_callback('q', |s| s.quit());
    root.add_global_callback('h', |_| todo!("Help menu"));

    root.run();
    Ok(())
}
