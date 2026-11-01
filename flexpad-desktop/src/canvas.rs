use crate::profile::Tile;
use crate::ui::inspector::pin_summary;
use iced::widget::canvas::{self, Action, Canvas, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    SelectAndDrag(Option<String>),
    DragPosition(f32, f32),
    EndDrag,
}

pub struct GridProgram {
    pub tiles: Vec<Tile>,
    pub selected_tile_id: Option<String>,
    pub dragging_tile_id: Option<String>,
    pub highlighted_tile_ids: Vec<String>,
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
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let is_dark = theme.extended_palette().is_dark;
        let background_color = if is_dark {
            Color::from_rgb(0.08, 0.09, 0.11)
        } else {
            Color::from_rgb(0.94, 0.95, 0.96)
        };
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), background_color);

        let dot_color = if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.15)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.2)
        };
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
            let is_highlighted = self.highlighted_tile_ids.contains(&tile.id);
            let box_color = if is_highlighted {
                Color::from_rgb8(0x2F, 0x6B, 0x4E)
            } else if is_selected {
                Color::from_rgba(0.25, 0.5, 0.9, 0.9)
            } else {
                Color::from_rgba(0.15, 0.3, 0.6, 0.9)
            };
            frame.fill(&box_path, box_color);

            let border_color = if is_highlighted {
                Color::from_rgb8(0x86, 0xBC, 0x9C)
            } else if is_selected {
                Color::from_rgb(0.7, 0.9, 1.0)
            } else {
                Color::from_rgb(0.4, 0.6, 1.0)
            };

            let border_stroke = Stroke {
                style: canvas::stroke::Style::Solid(border_color),
                width: if is_highlighted {
                    3.5
                } else if is_selected {
                    2.5
                } else {
                    1.5
                },
                ..Stroke::default()
            };
            frame.stroke(&box_path, border_stroke);

            let pins_summary = pin_summary(tile);

            let display_name = shorten(&tile.display_name);

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
    tiles: Vec<Tile>,
    selected_tile_id: Option<String>,
    dragging_tile_id: Option<String>,
    highlighted_tile_ids: Vec<String>,
) -> Element<'static, CanvasMessage> {
    Canvas::new(GridProgram {
        tiles,
        selected_tile_id,
        dragging_tile_id,
        highlighted_tile_ids,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn shorten(name: &str) -> String {
    if name.chars().count() > 14 {
        let start: String = name.chars().take(12).collect();
        format!("{start}...")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Names

    #[test]
    fn shorten_keeps_short_names_and_cuts_on_characters() {
        assert_eq!(shorten("Button 1"), "Button 1");
        assert_eq!(shorten("Lautstärke links"), "Lautstärke l...");
        assert_eq!(shorten("ääääääääääääääää"), "ääääääääääää...");
    }
}
