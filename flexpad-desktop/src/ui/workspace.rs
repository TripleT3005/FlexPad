use iced::widget::{container, row, scrollable};
use iced::{Element, Length};
use crate::app::{Message, FlexPadApp};
use crate::canvas;
use crate::ui::inspector::view_inspector;

pub fn view_workspace(app: &FlexPadApp) -> Element<'_, Message> {
    let canvas_view = canvas::view_canvas(
        app.tiles.clone(), 
        app.selected_tile_id.clone(), 
        app.dragging_tile_id.clone()
    ).map(Message::CanvasEvent);

    let inspector_panel = container(scrollable(view_inspector(app)))
        .width(Length::Fixed(360.0))
        .height(Length::Fill)
        .padding(15);

    row![
        container(canvas_view).width(Length::Fill).height(Length::Fill),
        inspector_panel,
    ]
    .spacing(15)
    .into()
}