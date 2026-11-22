use std::ops::Deref;

use eyre::{Context, Result};
use rkyv::{Archive, Serialize};

use crate::{
    blob::Directory,
    manifest::{ArchivedManifest, Manifest},
};

pub mod blob;
pub mod manifest;

#[derive(Archive, Serialize)]
#[rkyv(archived = Project)]
pub struct ProjectBuilder {
    pub assets: Directory,
    manifest: Manifest,
}

impl Deref for Project {
    type Target = ArchivedManifest;
    fn deref(&self) -> &Self::Target {
        &self.manifest
    }
}

impl Project {
    pub fn load_from_bytes<'a>(bytes: &'a [u8]) -> Result<&'a Self> {
        rkyv::access::<Self, rkyv::rancor::Error>(bytes).context("Data decoding error")
    }

    #[rustfmt::skip]
    pub const fn is_debug() -> bool {
        #[cfg(debug_assertions)] { true }
        #[cfg(not(debug_assertions))] { false }
    }
}
