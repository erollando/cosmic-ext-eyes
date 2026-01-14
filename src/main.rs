mod global_cursor;
mod eyes;
mod persist;

use cosmic::app::{Core, Task};
use cosmic::iced::event::{self, Event};
use cosmic::iced::mouse;
use cosmic::iced::{Point, Size, Subscription, Vector};
use cosmic::Element;

use std::time::{Duration, Instant};

const APP_ID: &str = "com.xinia.CosmicAppletEyes";

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    cosmic::applet::run::<EyesApplet>(())
}

#[derive(Debug, Clone)]
enum Message {
    CursorMoved(Point),
    CursorLeft,
    GlobalCursor(global_cursor::Update),
    Tick(Instant),
}

struct EyesApplet {
    core: Core,
    last_tick: Instant,
    hover_cursor: Option<Point>,
    global_cursor: Option<Timed<global_cursor::Sample>>,
    global_to_local_offset: Option<Vector>,
    offset_dirty: bool,
    window_size: Size,
    eyes: eyes::Eyes,
}

#[derive(Clone, Copy, Debug)]
struct Timed<T> {
    value: T,
    at: Instant,
}

impl Default for EyesApplet {
    fn default() -> Self {
        Self {
            core: Core::default(),
            last_tick: Instant::now(),
            hover_cursor: None,
            global_cursor: None,
            global_to_local_offset: None,
            offset_dirty: false,
            window_size: Size::new(1.0, 1.0),
            eyes: eyes::Eyes::new(),
        }
    }
}

impl cosmic::Application for EyesApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        let mut app = Self {
            core,
            ..Self::default()
        };
        let scale = app.core.scale_factor().max(0.1);
        app.global_to_local_offset = persist::load_offset(scale);
        (app, Task::none())
    }

    fn on_window_resize(&mut self, _id: cosmic::iced::window::Id, width: f32, height: f32) {
        self.window_size = Size::new(width.max(1.0), height.max(1.0));
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            cosmic::iced::event::listen_with(|event, status, _id| {
                if let event::Status::Captured = status {
                    return None;
                }

                match event {
                    Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        Some(Message::CursorMoved(position))
                    }
                    Event::Mouse(mouse::Event::CursorLeft) => Some(Message::CursorLeft),
                    _ => None,
                }
            }),
            global_cursor::subscription().map(Message::GlobalCursor),
            cosmic::iced::time::every(Duration::from_millis(1000 / 60)).map(Message::Tick),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CursorMoved(position) => {
                let now = Instant::now();
                self.hover_cursor = Some(position);
                self.maybe_calibrate(now, position);
            }
            Message::CursorLeft => {
                self.hover_cursor = None;
                if self.offset_dirty {
                    if let Some(offset) = self.global_to_local_offset {
                        let scale = self.core.scale_factor().max(0.1);
                        let _ = persist::save_offset(scale, offset);
                    }
                    self.offset_dirty = false;
                }
            }
            Message::GlobalCursor(sample) => {
                match sample {
                    global_cursor::Update::Position(sample) => {
                        self.global_cursor = Some(Timed { value: sample, at: Instant::now() });
                    }
                    global_cursor::Update::Left => {
                        self.global_cursor = None;
                    }
                }
            }
            Message::Tick(now) => {
                let dt = (now - self.last_tick).as_secs_f32();
                self.last_tick = now;

                let cursor = if let Some(local) = self.hover_cursor {
                    Some(local)
                } else if let (Some(Timed { value: sample, .. }), Some(offset)) =
                    (self.global_cursor, self.global_to_local_offset)
                {
                    let scale = self.core.scale_factor().max(0.1);
                    let global = Point::new(sample.x / scale, sample.y / scale);
                    Some(Point::new(global.x - offset.x, global.y - offset.y))
                } else {
                    None
                };

                self.eyes.tick(cursor, self.window_size, dt);
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.eyes.view()
    }

    fn view_window(&self, _id: cosmic::iced::window::Id) -> Element<'_, Message> {
        self.view()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

impl EyesApplet {
    fn maybe_calibrate(&mut self, now: Instant, local: Point) {
        const MAX_SKEW: Duration = Duration::from_millis(80);
        let Some(Timed { value: global, at: global_at }) = self.global_cursor else {
            return;
        };
        if now.saturating_duration_since(global_at) > MAX_SKEW {
            return;
        }

        let scale = self.core.scale_factor().max(0.1);
        let global_logical = Point::new(global.x / scale, global.y / scale);
        let new_offset = Vector::new(global_logical.x - local.x, global_logical.y - local.y);

        // Smooth to reduce jitter from timing mismatch.
        self.global_to_local_offset = Some(if let Some(old) = self.global_to_local_offset {
            let t = 0.25;
            Vector::new(old.x + (new_offset.x - old.x) * t, old.y + (new_offset.y - old.y) * t)
        } else {
            new_offset
        });
        self.offset_dirty = true;

        if std::env::var_os("COSMIC_EYES_DEBUG").is_some() {
            tracing::info!(
                scale,
                global_x = global.x,
                global_y = global.y,
                global_logical_x = global_logical.x,
                global_logical_y = global_logical.y,
                local_x = local.x,
                local_y = local.y,
                offset_x = self.global_to_local_offset.unwrap().x,
                offset_y = self.global_to_local_offset.unwrap().y,
                "calibrated global->local offset"
            );
        }
    }
}
