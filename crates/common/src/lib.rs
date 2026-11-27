use std::ops::Deref;

#[cfg(feature = "blob-write")]
use camino::Utf8Path;
#[cfg(feature = "blob-write")]
use eyre::ContextCompat;
use eyre::{Context, Result};
use rkyv::{Archive, Serialize};
#[cfg(feature = "blob-write")]
use rkyv::{rancor, util::AlignedVec};

use crate::blob::Directory;

pub mod blob;

pub const RESOURCE_ID: &str = "CONDUIT";

#[derive(Archive, Serialize)]
pub struct Manifest {
    pub name: String,
    pub version: String, // TODO: read from windows' native resource
    pub release: bool,
    pub index: String,
    pub width: u16,
    pub height: u16,
    pub custom_cursor: bool,
}

#[derive(Archive, Serialize)]
#[rkyv(archived = ProjectData)]
pub struct ProjectBuilder {
    pub assets: Directory,
    manifest: Manifest,
}

impl Deref for ProjectData {
    type Target = ArchivedManifest;
    fn deref(&self) -> &Self::Target {
        &self.manifest
    }
}

impl ProjectData {
    pub fn load_from_bytes<'a>(bytes: &'a [u8]) -> Result<&'a Self> {
        rkyv::access::<Self, rkyv::rancor::Error>(bytes).context("Data decoding error")
    }

    pub fn is_debug(&self) -> bool {
        !self.release
    }
}

pub type Project = &'static ProjectData;

#[cfg(feature = "blob-write")]
pub fn bundle_project(path: &std::path::Path, manifest: Manifest) -> Result<AlignedVec> {
    let path = Utf8Path::from_path(path).context("Only UTF-8 paths are supported")?;
    let assets = blob::write::encode_dir(path)?;

    let builder = ProjectBuilder { manifest, assets };

    rkyv::to_bytes::<rancor::Error>(&builder).context("Failed to serialize")
}
