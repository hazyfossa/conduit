#![windows_subsystem = "windows"]
use std::sync::OnceLock;

use conduit_common::{Project, ProjectData, RESOURCE_ID};
use eyre::{Context, OptionExt, Result};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Theme, Window, WindowId},
};

use winwin::blob::{ResourceKind, current_resources};
use wry::{
    NewWindowFeatures, NewWindowResponse, Rect, WebView, WebViewBuilder,
    dpi::{LogicalPosition, LogicalSize},
};

use crate::{
    actions::{ActionEvent, create_ipc_host},
    assets::create_asset_handler,
    storage::create_storage_handler,
    utils::{
        warn::WarnExt,
        window::{lock_aspect_ratio, maybe_fullscreen},
    },
};

mod actions;
mod assets;
mod storage;
mod utils;

fn handle_window_request(url: String, _features: NewWindowFeatures) -> NewWindowResponse {
    tracing::error!(
        "Unexpected new-window request for: {url}. Currently, conduit only supports one window."
    );
    NewWindowResponse::Deny
}

fn load_project_blob() -> Result<Project> {
    let project_resource = current_resources()
        .open(ResourceKind::Data, RESOURCE_ID)
        .ok_or_eyre("Primary resource not found")?;

    let project_bytes = project_resource
        .load_bytes()
        .context("Failed to load primary resource")?;

    ProjectData::load_from_bytes(project_bytes).context("Primary resource is corrupted")
}

pub struct App {
    project: Project,
    window: Option<Window>,
    webview: Option<WebView>,
}

impl App {
    fn load(project: Project) -> Self {
        const CONDUIT_VERSION: &str = env!("CARGO_PKG_VERSION");
        const CONDUIT_BUILD_ID: u64 = const_random::const_random!(u64);

        let build_kind = if project.is_debug() {
            "debug"
        } else {
            "release"
        };

        let name = &project.name;
        let version = &project.version;

        tracing::info!(
            "Running [{name}-{version}-{build_kind}] using [conduit-{CONDUIT_VERSION}-{CONDUIT_BUILD_ID:x}]"
        );

        Self {
            project,
            window: None.into(),
            webview: None,
        }
    }

    fn create_webview(&mut self) -> Result<()> {
        let window = &self.current_window()?;

        let index_url = format!("bundled://{}", self.project.index);

        let ipc_host = create_ipc_host();
        let asset_handler = create_asset_handler(self.project);
        let storage_handler = create_storage_handler(self.project);

        let webview_settings = WebViewBuilder::new()
            .with_url(index_url)
            .with_ipc_handler(ipc_host)
            .with_custom_protocol("bundled".to_string(), asset_handler)
            .with_asynchronous_custom_protocol("state".to_string(), storage_handler)
            .with_new_window_req_handler(handle_window_request)
            .with_devtools(self.project.is_debug());

        let webview = webview_settings.build(window)?;
        self.webview.replace(webview);

        Ok(())
    }

    pub fn current_window(&self) -> Result<&Window> {
        self.window
            .as_ref()
            .ok_or_eyre("No window is currently open")
    }
}

impl ApplicationHandler<ActionEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let title = self.project.name.to_string();
        let size = PhysicalSize::new(
            self.project.width.to_native(),
            self.project.height.to_native(),
        );

        let fullscreen = false; // TODO: settings

        let window_attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(size)
            // .with_focused(true)
            .with_theme(Some(Theme::Dark)) // TODO
            .with_fullscreen(maybe_fullscreen(fullscreen));

        let window = event_loop
            .create_window(window_attributes)
            .context("Failed to create window")
            .unwrap();

        lock_aspect_ratio(&window);

        if self.project.custom_cursor {
            window.set_cursor_visible(false);
        }

        self.window.replace(window);

        self.create_webview()
            .context("Failed to create webview")
            .unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                let window = self.window.as_ref().unwrap();
                let webview = self.webview.as_ref().unwrap();

                let size = size.to_logical::<u32>(window.scale_factor());

                webview
                    .set_bounds(Rect {
                        position: LogicalPosition::new(0, 0).into(),
                        size: LogicalSize::new(size.width, size.height).into(),
                    })
                    .unwrap();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: ActionEvent) {
        event.handle(self).warn();
    }
}

pub fn init_tracing() {
    use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
    use winwin::tracing_layer::WinDebug;

    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(WinDebug)
        .with_filter(WinDebug);

    tracing_subscriber::registry().with(layer).init();
}

pub static EVENT_LOOP: OnceLock<EventLoopProxy<ActionEvent>> = OnceLock::new();

fn init_event_loop() -> Result<EventLoop<ActionEvent>> {
    let event_loop = EventLoop::with_user_event()
        .build()
        .context("Failed to build event loop")?;

    let proxy = event_loop.create_proxy();
    let _ = EVENT_LOOP.set(proxy);

    Ok(event_loop)
}

fn main() -> Result<()> {
    init_tracing();

    let project = load_project_blob()?;
    let mut app = App::load(project);

    let event_loop = init_event_loop()?;
    event_loop.run_app(&mut app)?;

    Ok(())
}
