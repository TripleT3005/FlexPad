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

    iced::application(FlexPadApp::new, FlexPadApp::update, FlexPadApp::view)
        .title("FlexPad")
        .window(iced::window::Settings {
            icon,
            size: iced::Size::new(1180.0, 780.0),
            min_size: Some(iced::Size::new(1060.0, 640.0)),
            ..iced::window::Settings::default()
        })
        .subscription(FlexPadApp::subscription)
        .run()
}
