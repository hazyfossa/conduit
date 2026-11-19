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

    use wry::http::{Response, StatusCode};

    pub type Data = Cow<'static, [u8]>;

    pub fn http_err(status: StatusCode) -> Response<Data> {
        let empty = Data::Owned(Vec::new());
        Response::builder().status(status).body(empty).unwrap()
    }
}

pub mod window {
    use tao::window::{Fullscreen, Window};

    pub trait WindowExt {
        fn set_fullscreen_simple(&self, value: bool);
    }

    impl WindowExt for Window {
        fn set_fullscreen_simple(&self, value: bool) {
            self.set_fullscreen(value.then_some(Fullscreen::Borderless(None)));
        }
    }
}
