use std::sync::{LazyLock, OnceLock};

use conduit_common::Project;
use eyre::{Context, Result};
use tao::{
    dpi::PhysicalSize,
    event::{Event as SystemEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Theme, Window, WindowBuilder},
};
use wry::{NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder};

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

static PROJECT: LazyLock<&'static Project> = LazyLock::new(|| {
    const CONDUIT_VERSION: &str = env!("CARGO_PKG_VERSION");
    const CONDUIT_BUILD_ID: u64 = const_random::const_random!(u64);

    let build_kind = if Project::is_debug() {
        "debug"
    } else {
        "release"
    };

    let project = winwin::blob::load_resouse_blob("CONDUIT")
        .map_err(eyre::Error::from)
        .and_then(Project::load_from_bytes)
        .expect("Failed to load bundled project");

    let name = &project.name;
    let version = &project.version;

    tracing::info!(
        "Running [{name}-{version}-{build_kind}] using [conduit-{CONDUIT_VERSION}-{CONDUIT_BUILD_ID:x}]"
    );

    project
});

fn create_window(event_loop: &EventLoop<()>) -> Result<&Window> {
    let title = PROJECT.name.to_string();
    let size = PhysicalSize::new(PROJECT.width.to_native(), PROJECT.height.to_native());
    let fullscreen = false; // TODO: settings

    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .with_focused(true)
        .with_theme(Some(Theme::Dark)) // TODO
        .with_fullscreen(maybe_fullscreen(fullscreen))
        .build(event_loop)
        .context("Failed to create window")?;

    // TODO: confirm no performance impact from winwin on main loop
    lock_aspect_ratio(&window);

    if PROJECT.custom_cursor {
        window.set_cursor_visible(false);
    }

    let _ = WINDOW.set(window);
    Ok(current_window())
}

fn create_webview() -> Result<WebView> {
    let ipc_host = create_ipc_host();
    let asset_handler = create_asset_handler();
    let storage_handler = create_storage_handler();

    let webview_settings = WebViewBuilder::new()
        .with_url(location_url(0))
        .with_ipc_handler(ipc_host)
        .with_custom_protocol("bundled".to_string(), asset_handler)
        .with_asynchronous_custom_protocol("state".to_string(), storage_handler)
        .with_new_window_req_handler(handle_window_request)
        .with_devtools(Project::is_debug());

    webview_settings
        .build(current_window())
        .context("Webview init failed")
}

fn run_loop(event_loop: EventLoop<()>) -> ! {
    event_loop.run(move |event, _t, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            SystemEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    })
}

fn init_tracing() {
    tracing_subscriber::fmt().init();
}

fn location_url(id: u8) -> String {
    format!("bundled://location{id}/index.html")
}

fn main() -> Result<()> {
    init_tracing();

    let event_loop = EventLoopBuilder::new().build();

    let _window = create_window(&event_loop)?;
    let _webview = create_webview();

    run_loop(event_loop)
}
