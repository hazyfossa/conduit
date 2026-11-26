#![windows_subsystem = "windows"]
use std::sync::OnceLock;

use conduit_common::{Project, ProjectData};
use eyre::{Context, OptionExt, Result};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Theme, Window, WindowId},
};

use winwin::blob::ResourceKind;
use wry::{
    NewWindowFeatures, NewWindowResponse, Rect, WebView, WebViewBuilder,
    dpi::{LogicalPosition, LogicalSize},
};

use crate::{
    actions::create_ipc_host,
    assets::create_asset_handler,
    storage::create_storage_handler,
    utils::window::{lock_aspect_ratio, maybe_fullscreen},
};

mod actions;
mod assets;
mod storage;
mod utils;

static WINDOW: OnceLock<Window> = OnceLock::new();
fn current_window() -> &'static Window {
    WINDOW.get().expect("Primary window not initialized")
}

fn handle_window_request(url: String, _features: NewWindowFeatures) -> NewWindowResponse {
    tracing::error!(
        "Unexpected new-window request for: {url}. Currently, conduit only supports one window."
    );
    NewWindowResponse::Deny
}

fn load_project_blob() -> Result<Project> {
    let exe = winwin::blob::CurrentExe::open();

    let project_resource = exe
        .resources()
        .open(ResourceKind::Data, "CONDUIT")
        .ok_or_eyre("Primary resource not found")?;

    let project_bytes = project_resource
        .load_bytes()
        .context("Failed to load primary resource")?;

    ProjectData::load_from_bytes(project_bytes).context("Primary resource is corrupted")
}

struct App {
    project: Project,
    window: Option<Window>,
    webview: Option<WebView>,
}

impl App {
    fn load() -> Result<Self> {
        const CONDUIT_VERSION: &str = env!("CARGO_PKG_VERSION");
        const CONDUIT_BUILD_ID: u64 = const_random::const_random!(u64);

        let build_kind = if ProjectData::is_debug() {
            "debug"
        } else {
            "release"
        };

        let project = load_project_blob()?;

        let name = &project.name;
        let version = &project.version;

        tracing::info!(
            "Running [{name}-{version}-{build_kind}] using [conduit-{CONDUIT_VERSION}-{CONDUIT_BUILD_ID:x}]"
        );

        Ok(Self {
            project,
            window: None,
            webview: None,
        })
    }

    fn create_webview(&mut self) -> Result<()> {
        let window = &self
            .window
            .as_ref()
            .ok_or_eyre("No window is currently open")?;

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
            .with_devtools(ProjectData::is_debug());

        let webview = webview_settings.build(window)?;
        self.webview.replace(webview);

        Ok(())
    }
}

impl ApplicationHandler for App {
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
}

fn init_tracing() {
    use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
    use winwin::tracing_layer::WinDebug;

    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(WinDebug)
        .with_filter(WinDebug);

    tracing_subscriber::registry().with(layer).init();
}

fn main() -> Result<()> {
    init_tracing();

    let event_loop = EventLoop::new().context("Failed to build event loop")?;
    let mut app = App::load()?;
    event_loop.run_app(&mut app)?;

    Ok(())
}
