use eyre::{Context, Result};
use serde::Deserialize;
use wry::{dpi::PhysicalSize, http::Request};

use crate::{
    App, EVENT_LOOP,
    utils::{warn::WarnExt, window::maybe_fullscreen},
};

pub type ActionEvent = Box<dyn Action>;

pub fn create_ipc_host() -> impl Fn(Request<String>) {
    let event_loop = EVENT_LOOP.get().unwrap();

    move |message: Request<String>| {
        let action: Option<Box<dyn Action>> = serde_json::from_str(message.body())
            .context("Invalid action message")
            .warn();

        if let Some(action) = action {
            let _ = event_loop.send_event(action);
        }
    }
}

#[typetag::deserialize(tag = "target")]
pub trait Action: Send + Sync + 'static {
    fn handle(&self, app: &mut App) -> Result<()>;
}

#[derive(Deserialize)]
pub enum WindowAction {
    Resize { width: u16, height: u16 },
    SetFullscreen(bool),
    SetMinimized(bool),
    SetTitle(String),
}

#[typetag::deserialize]
impl Action for WindowAction {
    fn handle(&self, app: &mut App) -> Result<()> {
        let window = app.current_window()?;

        match self {
            WindowAction::Resize { width, height } => {
                // best-effort
                let _ = window.request_inner_size(PhysicalSize::new(*width, *height));
            }
            WindowAction::SetFullscreen(value) => {
                window.set_fullscreen(maybe_fullscreen(*value));
            }
            WindowAction::SetMinimized(value) => window.set_minimized(*value),
            WindowAction::SetTitle(title) => window.set_title(title),
        }

        Ok(())
    }
}
