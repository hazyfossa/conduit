#[allow(unused)]
pub mod warn {
    use std::fmt::Debug;

    pub trait WarnExt<T> {
        fn warn(self) -> Option<T>;
    }

    impl<T, E: Debug> WarnExt<T> for std::result::Result<T, E> {
        fn warn(self) -> Option<T> {
            match self {
                Ok(value) => Some(value),
                Err(e) => {
                    tracing::warn!("{e:?}");
                    None
                }
            }
        }
    }
}

pub mod http {
    use std::borrow::Cow;

    use wry::http::{Request, Response, StatusCode};

    pub type RequestData = Request<Vec<u8>>;
    pub type ResponseData = Response<Cow<'static, [u8]>>;

    pub fn http_err(status: StatusCode) -> ResponseData {
        let empty = Cow::Owned(Vec::new());
        Response::builder().status(status).body(empty).unwrap()
    }
}

pub mod window {
    use tao::window::{Fullscreen, Window};

    pub fn maybe_fullscreen(value: bool) -> Option<Fullscreen> {
        value.then_some(Fullscreen::Borderless(None))
    }

    pub fn lock_aspect_ratio(window: &Window) {
        let size = window.inner_size();
        let aspect_ratio = size.width as f64 / size.height as f64;
        winwin::aspect_ratio::lock_aspect_ratio(window, aspect_ratio)
    }
}
