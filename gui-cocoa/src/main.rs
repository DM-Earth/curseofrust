#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod util;

fn main() {
    #[cfg(not(target_os = "macos"))]
    unreachable!("unsupported platform");

    #[cfg(target_os = "macos")]
    return cacao::appkit::App::new("com.dm.earth.curseofrust", app::CorApp::new()).run();
}
