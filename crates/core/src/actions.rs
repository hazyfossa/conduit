use std::rc::Rc;

use tao::window::Window;
use wry::http::Request;

pub fn create_ipc_host(window: Rc<Window>) -> impl Fn(Request<String>) {
    move |message: Request<String>| todo!()
}
