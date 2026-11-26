use eyre::{Context, Result};
use serde::Deserialize;
use wry::{dpi::PhysicalSize, http::Request};

use crate::{
    current_window,
    utils::{warn::WarnExt, window::maybe_fullscreen},
};

pub fn create_ipc_host() -> impl Fn(Request<String>) {
    move |message: Request<String>| {
        let action: Option<Box<dyn Action>> = serde_json::from_str(message.body())
            .context("Invalid action message")
            .warn();

        if let Some(action) = action {
            action.handle().warn();
        }
    }
}

#[typetag::deserialize(tag = "target")]
pub trait Action {
    fn handle(&self) -> Result<()>;
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
    fn handle(&self) -> Result<()> {
        let window = current_window();

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
