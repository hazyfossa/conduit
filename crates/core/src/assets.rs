use crate::utils::http::{Data, http_err};
use conduit_blob::Assets;
use eyre::{Context, Result};
use winwin::blob::{ResourceId, load_resouse_blob};
use wry::http::{Method, Request, Response, StatusCode, header::CONTENT_TYPE};

pub fn load() -> Result<Assets<'static>> {
    let bytes = load_resouse_blob(ResourceId::Text("CONDUIT"))?;

    // TODO: consider using `unchecked` here
    Assets::from_bytes(bytes).context("Decoding failure")
}

type AssetRequest = Request<Vec<u8>>;

pub fn create_asset_handler() -> Result<impl Fn(&str, AssetRequest) -> Response<Data>> {
    let assets = load()?;

    let handler = move |_webview_id: &str, request: AssetRequest| {
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

        match assets.get(&path) {
            Some(file) => Response::builder()
                .header(CONTENT_TYPE, file.mimetype())
                .body(file.data.into())
                .unwrap(),

            None => http_err(StatusCode::NOT_FOUND),
        }
    };

    Ok(handler)
}
