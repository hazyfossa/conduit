use eyre::{Context, OptionExt, Result};

use crate::{blob::Assets, manifest::ArchivedManifest};

pub mod blob;
pub mod manifest;

fn zerocopy_open<T>(bytes: &[u8]) -> Result<&T>
where
    T: rkyv::Portable
        + for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, rkyv::rancor::Error>>,
{
    rkyv::access::<T, rkyv::rancor::Error>(bytes).context("Data decoding error")
}

pub struct Project {
    pub assets: Assets<'static>,
}

impl Project {
    pub fn load_from_bytes(bytes: &'static [u8]) -> Result<Self> {
        let dir = zerocopy_open(bytes)?;
        let assets = Assets { dir };
        Ok(Self { assets })
    }

    pub fn manifest(&self) -> Result<&'static ArchivedManifest> {
        let data = self
            .assets
            .get("/manifest")
            .ok_or_eyre("This project does not bundle a manifest")?
            .data();

        zerocopy_open(data)
    }
}
