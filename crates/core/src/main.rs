use std::{borrow::Cow, rc::Rc};

use eyre::{Context, Result};
use tao::{
    dpi::PhysicalSize,
    event::{Event as SystemEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Fullscreen, Theme, Window, WindowBuilder},
};
use wry::{
    NewWindowFeatures, NewWindowResponse, WebViewBuilder,
    http::{Response, StatusCode},
};

use crate::actions::create_ipc_host;

mod actions;
mod utils;

const PROJECT_NAME: &str = "TODO";

pub type Data = Cow<'static, [u8]>;

pub fn http_err(status: StatusCode) -> Response<Data> {
    let empty = Data::Owned(Vec::new());
    Response::builder().status(status).body(empty).unwrap()
}

mod assets {
    use super::{Data, http_err};
    use rust_embed::Embed;
    use wry::http::{Method, Request, Response, StatusCode, header::CONTENT_TYPE};

    #[derive(Embed)]
    #[folder = "../../test/assets/"]
    #[prefix = "/"]
    pub struct Assets;

    pub fn content_handler(_webview_id: &str, request: Request<Vec<u8>>) -> Response<Data> {
        if request.method() != Method::GET {
            return http_err(StatusCode::METHOD_NOT_ALLOWED);
        }

        let subproject = request.uri().host().unwrap_or_default();
        let resource = request.uri().path();

        // TODO: for an empty subproject (request resource from root),
        // this will likely fail because of unneccessary '/' prefix.
        // TODO: ponder whether making subprojects completely different
        // (trust) domains is the right call
        let mut path = format!("{subproject}/{resource}");
        if path.ends_with('/') {
            path.push_str("index.html");
        }

        match Assets::get(&path) {
            Some(content) => Response::builder()
                .header(CONTENT_TYPE, content.metadata.mimetype())
                .body(content.data)
                .unwrap(),

            None => http_err(StatusCode::NOT_FOUND),
        }
    }
}

fn handle_window_request(url: String, _features: NewWindowFeatures) -> NewWindowResponse {
    tracing::debug!("New window: {url}");
    NewWindowResponse::Allow
}

// Returns `DEBUG` flag
fn load_project() -> bool {
    #[cfg(debug_assertions)]
    const DEBUG: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG: bool = false;

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const BUILD_ID: u64 = const_random::const_random!(u64);

    let build_kind = if DEBUG { "debug" } else { "release" };

    tracing::info!("Running {PROJECT_NAME} {VERSION} {build_kind} build [{BUILD_ID:x}]");

    DEBUG
}

fn lock_aspect_ratio(window: &Window) {
    let size = window.inner_size();
    let aspect_ratio = size.width as f64 / size.height as f64;
    // winwin::window::lock_aspect_ratio(window, aspect_ratio)
}

fn create_window(event_loop: &EventLoop<()>, fullscreen: bool) -> Result<Rc<Window>> {
    const CONSTRUCT_WIDTH: u32 = 1920;
    const CONSTRUCT_HEIGHT: u32 = 1080;

    let window = WindowBuilder::new()
        .with_title(PROJECT_NAME)
        .with_inner_size(PhysicalSize::new(CONSTRUCT_WIDTH, CONSTRUCT_HEIGHT))
        .with_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)))
        .with_focused(true)
        .with_theme(Some(Theme::Dark))
        .build(event_loop)
        .context("Failed to create window")?;

    #[cfg(target_os = "windows")]
    if !fullscreen {
        // TODO: confirm no performance impact from wfix on main loop
        lock_aspect_ratio(&window);
    }

    // refcount is required since rustc cannot prove `window` is aliмe while `ipc_host`
    // is alive. We can rewrite without it by deferring window-requring parts of `ipc_host` to event_loop
    // over event_proxy, but that is not trivial to implement
    Ok(Rc::new(window))
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
    let is_debug = load_project();

    let event_loop = EventLoopBuilder::with_user_event().build();

    let window = create_window(&event_loop, false)?;

    let ipc_host = create_ipc_host(window.clone());

    // let storage = Storage::open_in_appdata(PROJECT_NAME).expect("Storage init failed");
    // let storage_protocol =
    //     move |_: &str, request, ret| persist::state_handler(&storage, request, ret);

    let webview_settings = WebViewBuilder::new()
        .with_url(location_url(0))
        .with_custom_protocol("bundled".to_string(), assets::content_handler)
        // .with_asynchronous_custom_protocol("state".to_string(), storage_protocol)
        .with_ipc_handler(ipc_host)
        .with_new_window_req_handler(handle_window_request)
        .with_devtools(is_debug);

    let _webview = webview_settings
        .build(&window)
        .context("Webview init failed")?;

    run_loop(event_loop)
}
