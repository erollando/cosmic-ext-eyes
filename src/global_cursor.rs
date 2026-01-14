use cosmic::iced::Subscription;
use cosmic::iced_futures::{futures, stream};

use futures::{SinkExt, StreamExt};

use cosmic_client_toolkit::{
    screencopy::{CaptureCursorSession, CaptureSource, Formats, ScreencopyHandler, ScreencopyState},
    wayland_client::{
        Connection, Dispatch, QueueHandle, Proxy,
        globals::{GlobalList, GlobalListContents, registry_queue_init},
        protocol::{wl_output, wl_pointer, wl_registry, wl_seat},
        WEnum,
    },
};

use std::os::{
    fd::{FromRawFd, RawFd},
    unix::net::UnixStream,
};

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum Update {
    Position(Sample),
    Left,
}

pub fn subscription() -> Subscription<Update> {
    let enabled = std::env::var_os("COSMIC_EYES_DISABLE_GLOBAL_CURSOR").is_none();
    let privileged_fd = std::env::var("X_PRIVILEGED_WAYLAND_SOCKET").ok();

    if !enabled || privileged_fd.is_none() {
        return Subscription::none();
    }

    Subscription::run_with_id(
        "cosmic-ext-eyes-global-cursor",
        stream::channel(32, move |mut output| async move {
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<Update>();

            let privileged_fd = privileged_fd.clone().unwrap();
            std::thread::spawn(move || {
                if let Err(err) = run(privileged_fd, tx) {
                    tracing::warn!("global cursor thread failed: {err}");
                }
            });

            while let Some(update) = rx.next().await {
                let _ = output.send(update).await;
            }
        }),
    )
}

fn run(
    privileged_fd: String,
    tx: futures::channel::mpsc::UnboundedSender<Update>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let desired_output_name = std::env::var("COSMIC_PANEL_OUTPUT").ok();

    let fd = privileged_fd.parse::<RawFd>()?;
    let socket = unsafe { UnixStream::from_raw_fd(fd) };
    let conn = Connection::from_socket(socket)?;
    let (globals, mut event_queue) = registry_queue_init::<CursorWatcher>(&conn)?;
    let qh = event_queue.handle();

    let mut watcher = CursorWatcher::new(tx, desired_output_name);
    watcher.bind_initial_globals(&globals, &qh);

    let _ = event_queue.roundtrip(&mut watcher);
    watcher.ensure_cursor_session(&qh);

    loop {
        if watcher.tx.is_closed() {
            break;
        }

        // If we fail to dispatch (disconnect), exit the thread.
        event_queue.blocking_dispatch(&mut watcher)?;

        // If an output arrives late, we may be able to start the session now.
        watcher.ensure_cursor_session(&qh);
    }

    Ok(())
}

struct OutputInfo {
    global_name: u32,
    output: wl_output::WlOutput,
    name: Option<String>,
}

struct CursorWatcher {
    screencopy: Option<ScreencopyState>,
    desired_output_name: Option<String>,

    outputs: Vec<OutputInfo>,
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,

    cursor_session: Option<CaptureCursorSession>,
    cursor_session_attempted: bool,

    tx: futures::channel::mpsc::UnboundedSender<Update>,
}

impl CursorWatcher {
    fn new(
        tx: futures::channel::mpsc::UnboundedSender<Update>,
        desired_output_name: Option<String>,
    ) -> Self {
        Self {
            screencopy: None,
            desired_output_name,
            outputs: Vec::new(),
            seat: None,
            pointer: None,
            cursor_session: None,
            cursor_session_attempted: false,
            tx,
        }
    }

    fn bind_initial_globals(&mut self, globals: &GlobalList, qh: &QueueHandle<Self>) {
        // Initialize screencopy state using the real globals list.
        self.screencopy = Some(ScreencopyState::new(globals, qh));

        let list = globals.contents().clone_list();
        for g in list {
            match g.interface.as_str() {
                "wl_output" => {
                    let version = g.version.min(wl_output::WlOutput::interface().version).min(4);
                    let output = globals.registry().bind(g.name, version, qh, g.name);
                    self.outputs.push(OutputInfo {
                        global_name: g.name,
                        output,
                        name: None,
                    });
                }
                "wl_seat" => {
                    if self.seat.is_none() {
                        let version = g.version.min(wl_seat::WlSeat::interface().version).min(5);
                        let seat = globals.registry().bind(g.name, version, qh, ());
                        self.seat = Some(seat);
                    }
                }
                _ => {}
            }
        }
    }

    fn best_output(&self) -> Option<&OutputInfo> {
        if let Some(desired) = self.desired_output_name.as_deref() {
            if let Some(found) = self
                .outputs
                .iter()
                .find(|o| o.name.as_deref() == Some(desired))
            {
                return Some(found);
            }
        }
        self.outputs.first()
    }

    fn ensure_cursor_session(&mut self, qh: &QueueHandle<Self>) {
        if self.cursor_session_attempted {
            return;
        }
        if self.cursor_session.is_some() {
            return;
        }

        let Some(pointer) = self.pointer.as_ref() else { return };
        let Some((output, output_name)) = self.best_output().map(|o| (o.output.clone(), o.name.clone())) else { return };
        let Some(screencopy) = self.screencopy.as_ref() else { return };

        let capturer = screencopy.capturer();
        let source = CaptureSource::Output(output);

        match capturer.create_cursor_session(
            &source,
            pointer,
            qh,
            cosmic_client_toolkit::screencopy::ScreencopyCursorSessionData::default(),
        ) {
            Ok(session) => {
                self.cursor_session = Some(session);
                self.cursor_session_attempted = true;
                tracing::info!(
                    output = output_name.as_deref().unwrap_or("<unknown>"),
                    "global cursor session started"
                );
            }
            Err(err) => {
                self.cursor_session_attempted = true;
                tracing::warn!("failed to start global cursor session: {err}");
            }
        }
    }
}

impl ScreencopyHandler for CursorWatcher {
    fn screencopy_state(&mut self) -> &mut ScreencopyState {
        self.screencopy
            .as_mut()
            .expect("screencopy state must be initialized before use")
    }

    fn init_done(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session: &cosmic_client_toolkit::screencopy::CaptureSession,
        _formats: &Formats,
    ) {
    }

    fn stopped(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session: &cosmic_client_toolkit::screencopy::CaptureSession,
    ) {
    }

    fn ready(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _screencopy_frame: &cosmic_client_toolkit::screencopy::CaptureFrame,
        _frame: cosmic_client_toolkit::screencopy::Frame,
    ) {
    }

    fn failed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _screencopy_frame: &cosmic_client_toolkit::screencopy::CaptureFrame,
        _reason: WEnum<cosmic_client_toolkit::screencopy::FailureReason>,
    ) {
    }

    fn cursor_position(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &CaptureCursorSession,
        x: i32,
        y: i32,
    ) {
        let _ = self.tx.unbounded_send(Update::Position(Sample {
            x: x as f32,
            y: y as f32,
        }));
    }

    fn cursor_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _cursor_session: &CaptureCursorSession,
    ) {
        let _ = self.tx.unbounded_send(Update::Left);
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for CursorWatcher {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // For now we only bind globals during initial setup.
    }
}

impl Dispatch<wl_output::WlOutput, u32> for CursorWatcher {
    fn event(
        state: &mut Self,
        proxy: &wl_output::WlOutput,
        event: wl_output::Event,
        global_name: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let info = state
            .outputs
            .iter_mut()
            .find(|o| o.global_name == *global_name && o.output.id() == proxy.id());

        let Some(info) = info else { return };

        match event {
            wl_output::Event::Name { name } => {
                info.name = Some(name);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for CursorWatcher {
    fn event(
        state: &mut Self,
        proxy: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_seat::Event::Capabilities { capabilities } => {
                if state.pointer.is_none() {
                    if let WEnum::Value(capabilities) = capabilities {
                        if capabilities.contains(wl_seat::Capability::Pointer) {
                            state.pointer = Some(proxy.get_pointer(qh, ()));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for CursorWatcher {
    fn event(
        _state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        _event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

cosmic_client_toolkit::delegate_screencopy!(CursorWatcher);
