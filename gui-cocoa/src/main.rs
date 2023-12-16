use cacao::appkit::App;

mod app;

fn main() {
    App::new("dmearth.cor.cocoa", app::CorApp::default()).run();
}
