use argh::FromArgs;

#[derive(FromArgs)]
/// Package a web application into a single .exe
struct Args {
    #[argh(option)]
    /// project's name, an arbitrary string
    name: String,
    /// project's version, an arbitrary string
    #[argh(option)]
    version: String, // TODO: read from windows' native resource
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

fn main() {
    let args: Args = argh::from_env();
}
