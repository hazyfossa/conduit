use eyre::Result;
use winwin::blob::{ResourceId, load_resouse_blob};

pub struct Assets {
    state: u128,
}

pub fn load() -> Result<Assets> {
    let bytes = load_resouse_blob(ResourceId::Text("CONDUIT"));
    todo!()
}
