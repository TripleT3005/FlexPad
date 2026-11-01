use iced::{Point, Size};
use serde::{Deserialize, Serialize};

// Constants

pub const MIN_WINDOW_SIZE: Size = Size::new(1060.0, 640.0);

#[cfg(windows)]
const TITLE_BAR_HEIGHT: i32 = 32;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub maximized: bool,
    #[serde(default)]
    pub screen_rect: Option<ScreenRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StartPosition {
    Centered,
    At(Point),
}

impl From<StartPosition> for iced::window::Position {
    fn from(position: StartPosition) -> Self {
        match position {
            StartPosition::Centered => Self::Centered,
            StartPosition::At(point) => Self::Specific(point),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitialWindow {
    pub size: Size,
    pub position: StartPosition,
    pub maximized: bool,
}

// Restore

pub fn initial_window(saved: Option<WindowGeometry>) -> InitialWindow {
    initial_window_with(saved, visibility)
}

pub fn initial_window_with(
    saved: Option<WindowGeometry>,
    visibility: impl Fn(Option<ScreenRect>) -> Visibility,
) -> InitialWindow {
    let default = InitialWindow {
        size: MIN_WINDOW_SIZE,
        position: StartPosition::Centered,
        maximized: false,
    };
    let Some(saved) = saved else {
        return default;
    };
    if !saved.width.is_finite() || !saved.height.is_finite() {
        return default;
    }
    let size = Size::new(
        saved.width.max(MIN_WINDOW_SIZE.width),
        saved.height.max(MIN_WINDOW_SIZE.height),
    );
    let has_position = saved.x.is_finite() && saved.y.is_finite();
    match visibility(saved.screen_rect) {
        Visibility::Visible if has_position => InitialWindow {
            size,
            position: StartPosition::At(Point::new(saved.x, saved.y)),
            maximized: saved.maximized,
        },
        Visibility::Unknown => InitialWindow {
            size,
            maximized: saved.maximized,
            ..default
        },
        _ => default,
    }
}

// Save

pub fn geometry(
    position: Option<Point>,
    size: Size,
    maximized: bool,
    raw_id: u64,
    previous: Option<WindowGeometry>,
) -> Option<WindowGeometry> {
    if maximized {
        return previous.map(|previous| WindowGeometry {
            maximized: true,
            ..previous
        });
    }
    let position = position?;
    Some(WindowGeometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: false,
        screen_rect: screen_rect(raw_id),
    })
}

// Platform

#[cfg(windows)]
fn screen_rect(raw_id: u64) -> Option<ScreenRect> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let found = unsafe { GetWindowRect(raw_id as _, &mut rect) } != 0;
    found.then_some(ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

#[cfg(not(windows))]
fn screen_rect(_raw_id: u64) -> Option<ScreenRect> {
    None
}

#[cfg(windows)]
fn visibility(rect: Option<ScreenRect>) -> Visibility {
    let Some(rect) = rect else {
        return Visibility::Unknown;
    };
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONULL, MonitorFromRect};

    let title_bar = RECT {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.top + TITLE_BAR_HEIGHT,
    };
    if unsafe { MonitorFromRect(&title_bar, MONITOR_DEFAULTTONULL) }.is_null() {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

#[cfg(not(windows))]
fn visibility(_rect: Option<ScreenRect>) -> Visibility {
    Visibility::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved() -> WindowGeometry {
        WindowGeometry {
            x: 100.0,
            y: 50.0,
            width: 1300.0,
            height: 800.0,
            maximized: false,
            screen_rect: Some(ScreenRect {
                left: 100,
                top: 50,
                right: 1400,
                bottom: 850,
            }),
        }
    }

    // Restore

    #[test]
    fn first_start_is_centered_at_the_minimum_size() {
        let window = initial_window_with(None, |_| Visibility::Visible);

        assert_eq!(window.size, MIN_WINDOW_SIZE);
        assert_eq!(window.position, StartPosition::Centered);
        assert!(!window.maximized);
    }

    #[test]
    fn a_visible_window_comes_back_where_it_was() {
        let window = initial_window_with(Some(saved()), |_| Visibility::Visible);

        assert_eq!(window.size, Size::new(1300.0, 800.0));
        assert_eq!(window.position, StartPosition::At(Point::new(100.0, 50.0)));
    }

    #[test]
    fn a_window_off_screen_falls_back_to_the_default() {
        let window = initial_window_with(Some(saved()), |_| Visibility::Hidden);

        assert_eq!(window.size, MIN_WINDOW_SIZE);
        assert_eq!(window.position, StartPosition::Centered);
    }

    #[test]
    fn without_a_screen_rectangle_the_window_is_centered() {
        let geometry = WindowGeometry {
            screen_rect: None,
            ..saved()
        };
        let window = initial_window_with(Some(geometry), |rect| {
            if rect.is_some() {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }
        });

        assert_eq!(window.position, StartPosition::Centered);
    }

    #[test]
    fn sizes_below_the_minimum_or_invalid_are_corrected() {
        let small = WindowGeometry {
            width: 200.0,
            height: f32::NAN,
            ..saved()
        };
        assert_eq!(
            initial_window_with(Some(small), |_| Visibility::Visible).size,
            MIN_WINDOW_SIZE
        );

        let narrow = WindowGeometry {
            width: 200.0,
            ..saved()
        };
        assert_eq!(
            initial_window_with(Some(narrow), |_| Visibility::Visible).size,
            Size::new(MIN_WINDOW_SIZE.width, 800.0)
        );
    }

    #[test]
    fn a_maximized_window_opens_maximized() {
        let geometry = WindowGeometry {
            maximized: true,
            ..saved()
        };

        assert!(initial_window_with(Some(geometry), |_| Visibility::Visible).maximized);
        assert!(!initial_window_with(Some(geometry), |_| Visibility::Hidden).maximized);
    }

    #[test]
    fn without_a_screen_check_only_the_size_is_restored() {
        let window = initial_window_with(Some(saved()), |_| Visibility::Unknown);

        assert_eq!(window.size, Size::new(1300.0, 800.0));
        assert_eq!(window.position, StartPosition::Centered);
    }

    // Save

    #[test]
    fn maximizing_keeps_the_previous_normal_size() {
        let geometry = geometry(
            Some(Point::new(0.0, 0.0)),
            Size::new(3000.0, 2000.0),
            true,
            0,
            Some(saved()),
        );

        assert_eq!(
            geometry,
            Some(WindowGeometry {
                maximized: true,
                ..saved()
            })
        );
    }

    #[test]
    fn a_normal_window_stores_position_and_size() {
        let geometry = geometry(
            Some(Point::new(10.0, 20.0)),
            Size::new(1200.0, 700.0),
            false,
            0,
            None,
        )
        .unwrap();

        assert_eq!((geometry.x, geometry.y), (10.0, 20.0));
        assert_eq!((geometry.width, geometry.height), (1200.0, 700.0));
        assert!(!geometry.maximized);
    }

    #[test]
    fn settings_without_screen_rectangle_still_load() {
        let geometry: WindowGeometry =
            serde_json::from_str(r#"{"x":1,"y":2,"width":1100,"height":700}"#).unwrap();

        assert_eq!(geometry.screen_rect, None);
        assert!(!geometry.maximized);
    }
}
