use cacao::appkit::App;

mod app;

fn main() {
    App::new("com.dm.earth.curseofrust", app::CorApp::new()).run();
}
