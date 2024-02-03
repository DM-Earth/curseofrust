#[cfg(target_os = "macos")]
mod app;
mod util;

fn main() {
    #[cfg(target_os = "macos")]
    cacao::appkit::App::new("com.dm.earth.curseofrust", app::CorApp::new()).run();
}
