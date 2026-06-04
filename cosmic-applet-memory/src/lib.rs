// SPDX-License-Identifier: GPL-3.0-only

mod localize;

use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::LazyLock;
use std::time::Duration;

use cosmic::{
    Element, Task, app,
    applet::padded_control,
    iced::{
        self, Color, Length, Pixels, Point, Radians, Rectangle, Subscription,
        mouse::Cursor,
        platform_specific::shell::commands::popup::{destroy_popup, get_popup},
        widget::{Column, canvas},
        window,
    },
    widget::{self, autosize, text},
};

use crate::localize::localize;

static AUTOSIZE_MAIN_ID: LazyLock<widget::Id> = LazyLock::new(|| widget::Id::new("autosize-main"));

pub fn run() -> cosmic::iced::Result {
    localize();
    cosmic::applet::run::<Memory>(())
}

#[derive(Default)]
struct Memory {
    core: cosmic::app::Core,
    popup: Option<window::Id>,
    /// Total physical memory, in kibibytes.
    mem_total: u64,
    /// Memory in use (total - available), in kibibytes.
    mem_used: u64,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    TogglePopup,
    Closed(window::Id),
}

impl Memory {
    /// Fraction of memory in use, clamped to `0.0..=1.0`.
    fn fraction(&self) -> f32 {
        if self.mem_total == 0 {
            0.0
        } else {
            (self.mem_used as f32 / self.mem_total as f32).clamp(0.0, 1.0)
        }
    }

    fn percent(&self) -> u32 {
        (self.fraction() * 100.0).round() as u32
    }

    /// Read `/proc/meminfo` and refresh the cached usage figures.
    fn refresh(&mut self) {
        let Ok(contents) = std::fs::read_to_string("/proc/meminfo") else {
            tracing::warn!("failed to read /proc/meminfo");
            return;
        };

        let mut total = 0u64;
        let mut available = 0u64;
        for line in contents.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            // Values are reported in kB; grab the leading integer.
            let kb = value
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            match key {
                "MemTotal" => total = kb,
                "MemAvailable" => available = kb,
                _ => {}
            }
        }

        self.mem_total = total;
        self.mem_used = total.saturating_sub(available);
    }
}

impl cosmic::Application for Memory {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.system76.CosmicAppletMemory";

    fn init(core: cosmic::app::Core, _flags: ()) -> (Self, app::Task<Message>) {
        let mut app = Self {
            core,
            ..Default::default()
        };
        app.refresh();
        (app, Task::none())
    }

    fn core(&self) -> &cosmic::app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::app::Core {
        &mut self.core
    }

    fn style(&self) -> Option<iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Message> {
        // Poll memory usage every two seconds.
        cosmic::iced::time::every(Duration::from_secs(2)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Message) -> app::Task<Message> {
        match message {
            Message::Tick => {
                self.refresh();
            }
            Message::TogglePopup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
                let new_id = window::Id::unique();
                self.popup.replace(new_id);
                let popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    new_id,
                    None,
                    None,
                    None,
                );
                return get_popup(popup_settings);
            }
            Message::Closed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // Size the ring to the panel's suggested icon size so it lines up with
        // the neighbouring applets.
        let (w, h) = self.core.applet.suggested_size(true);
        let size = w.max(h) as f32;

        let chart = canvas::Canvas::new(RingChart {
            fraction: self.fraction(),
        })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size));

        let button = widget::mouse_area(chart).on_press(Message::TogglePopup);

        autosize::autosize(button, AUTOSIZE_MAIN_ID.clone()).into()
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if self.popup != Some(id) {
            return widget::text("").into();
        }

        let used = self.mem_used as f64 / 1024.0 / 1024.0;
        let total = self.mem_total as f64 / 1024.0 / 1024.0;
        let space_xxs = self.core.system_theme().cosmic().space_xxs() as f32;

        let content = Column::with_children(vec![
            text::title4(fl!("memory")).into(),
            text::title3(format!("{}%", self.percent())).into(),
            text::body(format!("{used:.1} GiB of {total:.1} GiB used")).into(),
        ])
        .spacing(space_xxs);

        self.core
            .applet
            .popup_container(padded_control(content))
            .into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::Closed(id))
    }
}

/// A canvas program that strokes a circular usage ring: a faint full-circle
/// track with an accent-coloured arc on top proportional to `fraction`.
struct RingChart {
    fraction: f32,
}

impl canvas::Program<Message, cosmic::Theme> for RingChart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        let fg: Color = theme.cosmic().accent_color().into();
        let track_color = Color { a: 0.25, ..fg };

        // Leave room for the stroke so it isn't clipped at the bounds.
        let side = bounds.width.min(bounds.height);
        let thickness = (side * 0.13).max(2.0);
        let radius = (side - thickness) / 2.0;
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        // Background track.
        let track = canvas::Path::circle(center, radius);
        frame.stroke(
            &track,
            canvas::Stroke::default()
                .with_width(thickness)
                .with_color(track_color),
        );

        // Usage arc, drawn clockwise from the top (12 o'clock).
        if self.fraction > 0.0 {
            let start = -FRAC_PI_2;
            let arc = canvas::Path::new(|b| {
                b.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: Radians(start),
                    end_angle: Radians(start + self.fraction * TAU),
                });
            });
            frame.stroke(
                &arc,
                canvas::Stroke::default()
                    .with_width(thickness)
                    .with_color(fg)
                    .with_line_cap(canvas::LineCap::Round),
            );
        }

        // Usage percentage centred inside the ring, e.g. "45%".
        let label = format!("{}%", (self.fraction * 100.0).round() as u32);
        frame.fill_text(canvas::Text {
            content: label,
            position: center,
            color: fg,
            size: Pixels(side * 0.34),
            align_x: cosmic::iced::core::text::Alignment::Center,
            align_y: cosmic::iced::core::alignment::Vertical::Center,
            ..Default::default()
        });

        vec![frame.into_geometry()]
    }
}
