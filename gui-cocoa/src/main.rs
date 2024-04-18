#![cfg(target_os = "macos")]

mod app;
mod util;

fn main() {
    cacao::appkit::App::new("com.dm.earth.curseofrust", app::CorApp::new()).run();
}
