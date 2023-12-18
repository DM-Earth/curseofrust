use cacao::{appkit::{
    menu::{Menu, MenuItem},
    window::Window,
    App, AppDelegate,
}, image::Image};
#[derive(Default)]
pub struct CorApp {
    game_window: Window,
}

impl AppDelegate for CorApp {
    fn did_finish_launching(&self) {
        self.game_window.set_content_size(200, 150);
        self.game_window.set_title("corCocoa");
        self.game_window.show();
        App::set_menu(Self::menu());
        // Self::change_app_menu_name("CoR");
        App::activate();
    }
}

impl CorApp {
    fn menu() -> Vec<Menu> {
        vec![Menu::new(
            "corCocoa",
            vec![
                MenuItem::About("curseofrust".to_string()),
                MenuItem::Separator,
                MenuItem::Quit,
            ],
        )]
    }

    /// Lost main menu's bold style.
    fn change_app_menu_name(name: &str) {
        use cacao::foundation::NSString;
        let string: NSString = NSString::new(name);
        unsafe {
            use cacao::foundation::id;
            use cacao::objc::{class, msg_send, sel, sel_impl};
            let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
            let main_menu: id = msg_send![shared_app, mainMenu];
            let item_zero: id = msg_send![main_menu,itemAtIndex:0];
            let app_menu: id = msg_send![item_zero, submenu];
            let _: () = msg_send![app_menu ,setTitle:string];
        }
    }

    /// Very raw, very ugly.
    fn draw_and_set_app_menu_name(name: &str) {
        use cacao::foundation::NSString;
        let string: NSString = NSString::new(name);
        unsafe {
            use cacao::core_graphics::geometry::{CGPoint, CGRect, CGSize};
            use cacao::foundation::{id, nil, NSMutableDictionary};
            use cacao::objc::{class, msg_send, sel, sel_impl};

            let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
            let main_menu: id = msg_send![shared_app, mainMenu];
            let item_zero: id = msg_send![main_menu, itemAtIndex:0];
            let app_menu: id = msg_send![item_zero, submenu];

            let font: id = msg_send![class!(NSFont), boldSystemFontOfSize:13];
            let mut dict: NSMutableDictionary = NSMutableDictionary::new();
            dict.insert(NSString::new("NSFontAttributeName"), font);
            let dict_objc: id = dict.into_inner();
            let size: CGSize = msg_send![string, sizeWithAttributes:dict_objc];
            let mut image: id = msg_send![class!(NSImage), alloc];
            image = msg_send![image, initWithSize:size];
            let _: () = msg_send![image, lockFocus];
            let rect: CGRect = CGRect::new(&CGPoint::new(0.0, 0.5), &size);
            let _: () =
                msg_send![string, drawWithRect:rect options:1<<0 attributes:dict_objc context:nil];
            let _: () = msg_send![image, unlockFocus];

            let _: () = msg_send![app_menu, setTitle:NSString::new("")];
            let _: () = msg_send![item_zero, setImage:image];
        }
    }

    /// Icon is hard-coded, so call this only once.\
    /// Just modify this fn if you want to change icon.
    fn set_app_icon(){
        let image:Image=Image::with_data(include_bytes!("../images/icon.bmp"));
        todo!()
    }
}
