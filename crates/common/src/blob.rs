use std::cmp::Ordering;

use camino::{Utf8Component, Utf8Path};
use rkyv::{Archive, Serialize};

#[derive(Archive, Serialize)]
pub struct File {
    path: String,
    mime_type: Option<String>,
    data: Vec<u8>,
}

impl ArchivedFile {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn mimetype(&self) -> Option<&str> {
        self.mime_type.as_ref().map(|m| m.as_str())
    }
}

#[derive(Archive, Serialize)]
#[allow(unused, reason = "only the archived variant is used")]
pub struct Directory {
    entries: Vec<File>,
}

impl ArchivedDirectory {
    pub fn get(&self, path: &str) -> Option<&ArchivedFile> {
        let entries = &self.entries;

        let idx = entries
            .binary_search_by(|e| cmp_key_path(e.path.as_str(), &path))
            .ok()?;

        let file = &entries[idx];

        Some(file)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter_all(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.entries
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

    fn encode_file(root: &Utf8Path, path: Utf8PathBuf) -> Result<File> {
        let rel = path
            .strip_prefix(root)
            .context("walk escaped from directory root")?;

        let path = relative_key(rel);
        let mime_type = mime_guess2::from_path(&path).first().map(|m| m.to_string());

        let data = fs::read(&path)?;

        Ok(File {
            path,
            mime_type,
            data,
        })
    }

    pub fn encode_dir(root: &Utf8Path) -> Result<Directory> {
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

        Ok(Directory { entries })
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
