mod app;
mod canvas;
mod components;
mod config;
mod constants;
mod device;
mod profile;
mod ui;

use app::FlexPadApp;

fn main() -> iced::Result {
    iced::application(FlexPadApp::new, FlexPadApp::update, FlexPadApp::view)
        .title("FlexPad")
        .subscription(FlexPadApp::subscription)
        .run()
}
