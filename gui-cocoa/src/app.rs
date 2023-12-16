use cacao::appkit::{menu::Menu, window::Window, App, AppDelegate};

#[derive(Default)]
pub struct CorApp {
    window: Window,
}

impl AppDelegate for CorApp {
    fn did_finish_launching(&self) {
        self.window.set_content_size(200, 150);
        self.window.set_title("corCocoa");
        self.window.show();
        App::set_menu(Menu::standard());
        App::activate();
    }
}
