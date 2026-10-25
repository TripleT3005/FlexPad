mod app;
mod config;
mod device;
mod canvas;
mod ui;
mod constants;

use app::FlexPadApp;

fn main() -> iced::Result {
    iced::application(FlexPadApp::new, FlexPadApp::update, FlexPadApp::view)
        .title("FlexPad")
        .subscription(FlexPadApp::subscription)
        .run()
}