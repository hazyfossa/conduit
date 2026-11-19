use rkyv::{Archive, Serialize};

#[derive(Archive, Serialize)]
pub struct Manifest {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub resizable: bool,
    pub custom_cursor: bool,
}
