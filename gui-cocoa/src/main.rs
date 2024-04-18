#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod util;

fn main() {
    #[cfg(target_os = "macos")]
    return cacao::appkit::App::new("com.dm.earth.curseofrust", app::CorApp::new()).run();

    unreachable!("unsupported platform")
}
