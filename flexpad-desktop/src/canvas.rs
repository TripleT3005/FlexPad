use crate::config::CanvasTile;
use iced::widget::canvas::{self, Action, Canvas, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    SelectAndDrag(Option<String>),
    DragPosition(f32, f32),
    EndDrag,
}

pub struct GridProgram {
    pub tiles: Vec<CanvasTile>,
    pub selected_tile_id: Option<String>,
    pub dragging_tile_id: Option<String>,
}

impl Program<CanvasMessage> for GridProgram {
    type State = ();

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(cursor_pos) = cursor.position_in(bounds) {
            for tile in &self.tiles {
                let rect = Rectangle::new(Point::new(tile.x, tile.y), Size::new(100.0, 60.0));
                if rect.contains(cursor_pos) {
                    return mouse::Interaction::Pointer;
                }
            }
        }
        if self.dragging_tile_id.is_some() {
            return mouse::Interaction::Grabbing;
        }
        mouse::Interaction::default()
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<Action<CanvasMessage>> {
        let cursor_pos = cursor.position_in(bounds);

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor_pos {
                    let mut clicked_id = None;
                    for tile in self.tiles.iter().rev() {
                        let rect =
                            Rectangle::new(Point::new(tile.x, tile.y), Size::new(100.0, 60.0));
                        if rect.contains(pos) {
                            clicked_id = Some(tile.id.clone());
                            break;
                        }
                    }
                    return Some(
                        Action::publish(CanvasMessage::SelectAndDrag(clicked_id)).and_capture(),
                    );
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if self.dragging_tile_id.is_some()
                    && let Some(pos) = cursor_pos
                {
                    let snap: f32 = 30.0;
                    let new_x = (pos.x / snap).round() * snap;
                    let new_y = (pos.y / snap).round() * snap;
                    return Some(
                        Action::publish(CanvasMessage::DragPosition(
                            new_x.max(0.0),
                            new_y.max(0.0),
                        ))
                        .and_capture(),
                    );
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.dragging_tile_id.is_some() =>
            {
                return Some(Action::publish(CanvasMessage::EndDrag).and_capture());
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let background_color = Color::from_rgb(0.08, 0.09, 0.11);
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), background_color);

        let dot_color = Color::from_rgba(1.0, 1.0, 1.0, 0.15);
        let spacing: f32 = 30.0;
        let dot_radius: f32 = 1.5;

        let mut x = spacing;
        while x < bounds.width {
            let mut y = spacing;
            while y < bounds.height {
                frame.fill(&Path::circle(Point::new(x, y), dot_radius), dot_color);
                y += spacing;
            }
            x += spacing;
        }

        for tile in &self.tiles {
            let rect_size = Size::new(100.0, 60.0);
            let rect_pos = Point::new(tile.x, tile.y);

            let box_path = Path::rectangle(rect_pos, rect_size);

            let is_selected = self.selected_tile_id.as_ref() == Some(&tile.id);
            let box_color = if is_selected {
                Color::from_rgba(0.25, 0.5, 0.9, 0.9)
            } else {
                Color::from_rgba(0.15, 0.3, 0.6, 0.9)
            };
            frame.fill(&box_path, box_color);

            let border_color = if is_selected {
                Color::from_rgb(0.7, 0.9, 1.0)
            } else {
                Color::from_rgb(0.4, 0.6, 1.0)
            };

            let border_stroke = Stroke {
                style: canvas::stroke::Style::Solid(border_color),
                width: if is_selected { 2.5 } else { 1.5 },
                ..Stroke::default()
            };
            frame.stroke(&box_path, border_stroke);

            let pins_summary: String = tile
                .functions
                .values()
                .map(|f| {
                    if f.pin.is_empty() {
                        "-".to_string()
                    } else {
                        f.pin.clone()
                    }
                })
                .collect::<Vec<String>>()
                .join(", ");

            let display_name = if tile.display_name.len() > 14 {
                format!("{}...", &tile.display_name[..12])
            } else {
                tile.display_name.clone()
            };

            let text_content = format!("{}\nPins: {}", display_name, pins_summary);
            let text = canvas::Text {
                content: text_content,
                position: Point::new(rect_pos.x + 8.0, rect_pos.y + 10.0),
                color: Color::WHITE,
                size: iced::Pixels(12.0),
                ..canvas::Text::default()
            };
            frame.fill_text(text);
        }

        vec![frame.into_geometry()]
    }
}

pub fn view_canvas(
    tiles: Vec<CanvasTile>,
    selected_tile_id: Option<String>,
    dragging_tile_id: Option<String>,
) -> Element<'static, CanvasMessage> {
    Canvas::new(GridProgram {
        tiles,
        selected_tile_id,
        dragging_tile_id,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
