#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod components;
mod config;
mod constants;
mod device;
mod flasher;
mod icon;
mod keys;
mod macros;
mod profile;
mod release;
mod ui;
mod update;
mod window_state;

use app::FlexPadApp;

// Constants

const WINDOW_ICON_SIZE: u16 = 64;

// Entry point

fn main() -> iced::Result {
    let icon = iced::window::icon::from_rgba(
        icon::icon_pixels(WINDOW_ICON_SIZE),
        u32::from(WINDOW_ICON_SIZE),
        u32::from(WINDOW_ICON_SIZE),
    )
    .ok();
    let window = window_state::initial_window(window_state::load_window_geometry());

    iced::application(FlexPadApp::new, FlexPadApp::update, FlexPadApp::view)
        .title("FlexPad")
        .window(iced::window::Settings {
            icon,
            size: window.size,
            position: window.position.into(),
            maximized: window.maximized,
            min_size: Some(window_state::MIN_WINDOW_SIZE),
            exit_on_close_request: false,
            ..iced::window::Settings::default()
        })
        .subscription(FlexPadApp::subscription)
        .run()
}
