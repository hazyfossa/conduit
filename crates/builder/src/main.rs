use std::path::PathBuf;

use argh::FromArgs;
use conduit_common::{Manifest, RESOURCE_ID, bundle_project};
use eyre::{Context, Result};
use winwin::resources::{ResourceUpdateTx, resource_kind};

#[derive(FromArgs)]
/// Package a web application into a single .exe
struct Args {
    #[argh(option)]
    /// path to conduit-runtime.exe
    runtime: PathBuf,
    #[argh(option)]
    /// path to asset directory
    assets: PathBuf,
    #[argh(option)]
    /// path to target.exe
    target: PathBuf,

    #[argh(option)]
    /// project's name, an arbitrary string
    name: String,
    /// project's version, an arbitrary string
    #[argh(option)]
    version: String, // TODO: read from windows' native resource
    /// release flag
    #[argh(switch)]
    release: bool,
    /// index resource path. default is index.html
    #[argh(option)]
    index: String,
    /// initial window width
    #[argh(option)]
    width: u16,
    /// initial window height
    #[argh(option)]
    height: u16,
    /// whether your app displays a custom cursor. disables the native cursor
    #[argh(switch)]
    custom_cursor: bool,
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let manifest = Manifest {
        name: args.name,
        version: args.version,
        index: args.index,
        release: args.release,
        width: args.width,
        height: args.height,
        custom_cursor: args.custom_cursor,
    };

    let project_bundle =
        bundle_project(&args.assets, manifest).context("Failed to bundle project")?;

    std::fs::copy(&args.runtime, &args.target).context("Failed to write target.exe")?;

    let mut runtime = ResourceUpdateTx::open(&args.target, true)?;
    runtime.write::<resource_kind::Data>(RESOURCE_ID, &project_bundle)?;
    runtime.commit()?;

    Ok(())
}
