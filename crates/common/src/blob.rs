use std::cmp::Ordering;

use camino::{Utf8Component, Utf8Path};
use eyre::Result;
use rkyv::{Archive, Serialize, rancor};

#[derive(Archive, Serialize)]
struct Manifest {
    entries: Vec<FileEntry>,
}

#[derive(Archive, Serialize)]
struct FileEntry {
    path: String,
    mime_type: Option<String>,
    data: Vec<u8>,
}

pub struct Asset<'a> {
    file: &'a ArchivedFileEntry,
}

impl<'a> Asset<'a> {
    pub fn data(&self) -> &'a [u8] {
        &self.file.data
    }

    pub fn mimetype(&self) -> Option<&'a str> {
        self.file.mime_type.as_ref().map(|m| m.as_str())
    }
}

pub struct Assets<'a> {
    manifest: &'a ArchivedManifest,
}

impl<'a> Assets<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        let manifest = rkyv::access::<_, rancor::Error>(bytes)?;
        Ok(Self { manifest })
    }

    pub unsafe fn from_bytes_unchecked(bytes: &'a [u8]) -> Self {
        let manifest = unsafe { rkyv::access_unchecked(bytes) };
        Self { manifest }
    }

    pub fn get(&self, path: &str) -> Option<Asset<'a>> {
        let entries = &self.manifest.entries;

        let idx = entries
            .binary_search_by(|e| cmp_key_path(e.path.as_str(), &path))
            .ok()?;

        let file = &entries[idx];

        Some(Asset { file })
    }

    pub fn len(&self) -> usize {
        self.manifest.entries.len()
    }

    pub fn iter_all(&self) -> impl Iterator<Item = (&'a str, &'a [u8])> {
        self.manifest
            .entries
            .iter()
            .map(|e| (e.path.as_str(), e.data.as_slice()))
    }
}

fn cmp_key_path(key: &str, path: &str) -> Ordering {
    let path: &Utf8Path = path.into();
    let mut key_segs = key.split('/');
    for comp in path.components() {
        let seg = match comp {
            Utf8Component::Normal(s) => s,
            Utf8Component::CurDir => continue,
            _ => return Ordering::Greater,
        };
        match key_segs.next() {
            Some(k) => match k.cmp(seg) {
                Ordering::Equal => {}
                ord => return ord,
            },
            None => return Ordering::Less,
        }
    }
    if key_segs.next().is_some() {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

#[cfg(feature = "blob-write")]
pub mod write {
    use camino::Utf8PathBuf;
    use eyre::{Context, Result};
    use fs_err as fs;
    use rkyv::util::AlignedVec;

    use super::*;

    fn encode_file(root: &Utf8Path, path: Utf8PathBuf) -> Result<FileEntry> {
        let rel = path
            .strip_prefix(root)
            .context("walk escaped from directory root")?;

        let path = relative_key(rel);
        let mime_type = mime_guess2::from_path(&path).first().map(|m| m.to_string());

        let data = fs::read(&path)?;

        Ok(FileEntry {
            path,
            mime_type,
            data,
        })
    }

    pub fn encode_dir(root: &Utf8Path) -> Result<AlignedVec> {
        let mut entries = Vec::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let ty = entry.file_type()?;
                let path = Utf8PathBuf::try_from(entry.path()).context("Non-UTF8 path")?;

                if ty.is_dir() {
                    stack.push(path);
                } else if ty.is_file() {
                    let entry = encode_file(root, path)?;
                    entries.push(entry);
                }
            }
        }

        entries.sort_unstable_by(|a, b| a.path.split('/').cmp(b.path.split('/')));

        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&Manifest { entries })?;
        Ok(bytes)
    }

    fn relative_key(rel: &Utf8Path) -> String {
        let mut key = String::new();
        for comp in rel.components() {
            if let Utf8Component::Normal(part) = comp {
                if !key.is_empty() {
                    key.push('/');
                }
                key.push_str(part);
            }
        }
        key
    }
}
