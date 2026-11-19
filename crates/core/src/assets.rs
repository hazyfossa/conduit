use crate::{
    PROJECT,
    utils::http::{RequestData, ResponseData, http_err},
};
use wry::http::{HeaderValue, Method, Response, StatusCode, header::CONTENT_TYPE};

pub fn create_asset_handler() -> impl Fn(&str, RequestData) -> ResponseData {
    move |_webview_id: &str, request: RequestData| {
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

        match PROJECT.assets.get(&path) {
            Some(file) => {
                let mut response = Response::new(file.data().into());

                if let Some(mime_type) = file.mimetype() {
                    let header = HeaderValue::from_static(mime_type);
                    response.headers_mut().insert(CONTENT_TYPE, header);
                }

                response
            }

            None => http_err(StatusCode::NOT_FOUND),
        }
    }
}
