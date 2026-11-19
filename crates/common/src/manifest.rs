use rkyv::{Archive, Serialize};

#[derive(Archive, Serialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub width: u16,
    pub height: u16,
    pub custom_cursor: bool,
}
