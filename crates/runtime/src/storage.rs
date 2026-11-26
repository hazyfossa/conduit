use std::borrow::Cow;
use std::path::PathBuf;

use conduit_common::Project;
use eyre::Result;
use fjall::KeyspaceCreateOptions;
use wry::RequestAsyncResponder;
use wry::http::{Method, Response, StatusCode};

use crate::utils::http::{RequestData, ResponseData, http_err};

// NOTE: Storage currently panics on underlying fjall errors

pub struct Storage {
    db: fjall::Database,
}

impl Storage {
    pub fn open_in_appdata(project: Project) -> Result<Self> {
        let path = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .expect("Cannot get APPDATA path from environment");

        let project_name = project.name.as_str();
        let path = path.join(project_name).join("savefile");

        Self::open(path)
    }

    fn open(path: PathBuf) -> Result<Self> {
        let db = fjall::Database::builder(path).open()?;
        Ok(Self { db })
    }

    // Opens a save. If such a save does not exist, it will be created
    fn open_save(&self, name: &str) -> Save {
        let table = self
            .db
            .keyspace(name, KeyspaceCreateOptions::default)
            .expect("Storage error on open");
        Save { table }
    }

    // TODO: this is very memory-inefficient
    #[allow(unused)] // TODO: list saves, select save
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

struct StorageMessage {
    req: RequestData,
    ret: RequestAsyncResponder,
}

fn handle_message(storage: &Storage, request: RequestData) -> Option<ResponseData> {
    let Some(save_name) = request.uri().host() else {
        tracing::warn!("Storage interaction without a specified save, ignoring");
        return None;
    };

    // TODO: do not reopen saves on each access
    let save = storage.open_save(save_name);

    let key = request.uri().path();

    Some(match *request.method() {
        Method::GET => match save.read(key) {
            Some(data) => Response::new(Cow::Owned(data)),
            None => http_err(StatusCode::NOT_FOUND),
        },
        Method::POST => {
            let value = request.body();
            save.write(key, value);
            return None;
        }
        _ => http_err(StatusCode::METHOD_NOT_ALLOWED),
    })
}

pub fn create_storage_handler(
    project: Project,
) -> impl Fn(&str, RequestData, RequestAsyncResponder) {
    let (tx, rx) = std::sync::mpsc::channel::<StorageMessage>();

    std::thread::Builder::new()
        .name("storage".to_string())
        .spawn(move || {
            let storage = Storage::open_in_appdata(project).unwrap();
            for msg in rx.iter() {
                let ret = handle_message(&storage, msg.req);

                if let Some(ret) = ret {
                    msg.ret.respond(ret);
                }
            }
        })
        .expect("Failed to run storage thread");

    move |_webview_id: &str, req: RequestData, ret: RequestAsyncResponder| {
        tx.send(StorageMessage { req, ret })
            .expect("Cannot queue storage operation")
    }
}
