use std::borrow::Cow;
use std::path::PathBuf;

use eyre::Result;
use fjall::KeyspaceCreateOptions;
use wry::RequestAsyncResponder;
use wry::http::{Method, Request, Response, StatusCode};

use crate::PROJECT;
use crate::utils::http::http_err;

// NOTE: Storage currently panics on underlying fjall errors

pub struct Storage {
    db: fjall::Database,
}

impl Storage {
    pub fn open_in_appdata() -> Result<Self> {
        let path = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .expect("Cannot get APPDATA path from environment");

        let project_name = PROJECT.name.as_str();
        let path = path.join(project_name).join("savefile");

        Self::open(path)
    }

    fn open(path: PathBuf) -> Result<Self> {
        let db = fjall::Database::builder(path).open()?;
        Ok(Self { db })
    }

    // Opens a save. If such a save does not exist, it will be created
    fn open_save(&self, name: &str) -> Result<Save> {
        let table = self.db.keyspace(name, KeyspaceCreateOptions::default)?;
        Ok(Save { table })
    }

    // TODO: this is very memory-inefficient
    fn list_saves(&self) -> Vec<String> {
        let list = self.db.list_keyspace_names();

        list.iter()
            .filter(|name| &***name != "settings")
            .map(|save_name| save_name.to_string())
            .collect::<Vec<_>>()
    }
}

pub struct Save {
    table: fjall::Keyspace,
}

impl Save {
    #[inline]
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        let data = self.table.get(key).expect("Storage failure on read");
        data.map(|bytes| bytes.to_vec())
    }

    #[inline]
    fn write(&self, key: &str, value: &[u8]) {
        self.table
            .insert(key, value)
            .expect("Storage failure on write")
    }
}

pub fn create_storage_handler() -> Result<impl Fn(&str, Request<Vec<u8>>, RequestAsyncResponder)> {
    let storage = Storage::open_in_appdata()?;

    let handler =
        move |_webview_id: &str, request: Request<Vec<u8>>, ret: RequestAsyncResponder| {
            let Some(save_name) = request.uri().host() else {
                tracing::warn!("Storage interaction without a specified save, ignoring");
                return;
            };

            // TODO: do not reopen saves on each access
            let save = storage.open_save(save_name).unwrap(); // TODO: merge web-error from another project

            let key = request.uri().path();

            // TODO: never block the handling thread

            match *request.method() {
                Method::GET => match save.read(key) {
                    Some(data) => ret.respond(Response::new(Cow::Owned(data))),
                    None => ret.respond(http_err(StatusCode::NOT_FOUND)),
                },
                Method::POST => {
                    let value = request.body();
                    save.write(key, value);
                }
                _ => ret.respond(http_err(StatusCode::METHOD_NOT_ALLOWED)),
            }
        };

    Ok(handler)
}
