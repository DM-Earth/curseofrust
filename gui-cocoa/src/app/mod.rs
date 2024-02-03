use std::array::from_fn;
use std::sync::Once;
use std::time::UNIX_EPOCH;

use crate::util::{app_from_objc, OnceAssign};
use build_time::build_time_local;
use cacao::{
    appkit::{
        menu::{Menu, MenuItem},
        window::Window,
        window::{WindowConfig, WindowDelegate, WindowStyle},
        App, AppDelegate,
    },
    events::EventModifierFlag,
    foundation::{id, nil, NSString},
    image::Image,
    objc::{class, msg_send, sel, sel_impl},
    text::Label,
};
use curseofrust::state::State;
use curseofrust::{MAX_HEIGHT, MAX_WIDTH};

mod config;

pub struct CorApp {
    // View-associated
    game_window: Window,
    about_window: Window<AboutWindow>,
    config_window: Window<config::ConfigWindow>,
    // Game-associated
    state: OnceAssign<State>,
    tile_variant: OnceAssign<[[i16; MAX_WIDTH as usize]; MAX_HEIGHT as usize]>,
}

impl AppDelegate for CorApp {
    fn did_finish_launching(&self) {
        self.game_window.set_content_size(200, 150);
        self.game_window.set_title("corCocoa");
        self.game_window.show();
        App::set_menu(Self::menu());
        // Self::change_app_menu_name("CoR");
        App::activate();
        Self::set_app_icon();
    }
}

impl CorApp {
    pub fn new() -> Self {
        Self {
            game_window: Default::default(),
            about_window: Window::with(fixed_size_window_config(), AboutWindow::new()),
            config_window: Window::with(fixed_size_window_config(), config::ConfigWindow::new()),
            state: OnceAssign::new(),
            tile_variant: OnceAssign::new(),
        }
    }

    fn menu() -> Vec<Menu> {
        let about = MenuItem::new("About Curse of Rust").action(|| {
            let app = app_from_objc::<Self>();
            app.about_window.show();
        });
        let preferences = MenuItem::new("Preferences")
            .modifiers(&[EventModifierFlag::Command])
            .key(",")
            .action(|| {
                let app = app_from_objc::<Self>();
                app.config_window.show();
            });
        let save_config = MenuItem::new("Save Preferences")
            .modifiers(&[EventModifierFlag::Command])
            .key("s")
            .action(|| {
                let app = app_from_objc::<Self>();
                if app.config_window.is_key() {
                    todo!()
                }
            });
        let restore_default_config = MenuItem::new("Restore Default Preferences").action(|| {
            let app = app_from_objc::<Self>();
            if app.config_window.is_key() {
                todo!()
            }
        });
        vec![
            Menu::new(
                "CoR Cocoa",
                vec![
                    about,
                    MenuItem::Separator,
                    preferences,
                    MenuItem::Separator,
                    MenuItem::Quit,
                ],
            ),
            Menu::new(
                "File",
                vec![
                    MenuItem::CloseWindow,
                    MenuItem::Separator,
                    save_config,
                    restore_default_config,
                ],
            ),
        ]
    }

    /// Loses main menu's bold style.
    fn _change_app_menu_name(name: &str) {
        let string: NSString = NSString::new(name);
        unsafe {
            let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
            let main_menu: id = msg_send![shared_app, mainMenu];
            let item_zero: id = msg_send![main_menu, itemAtIndex:0];
            let app_menu: id = msg_send![item_zero, submenu];
            let _: () = msg_send![app_menu, setTitle:string];
        }
    }

    /// Very raw, very ugly.
    fn _draw_and_set_app_menu_name(name: &str) {
        let string: NSString = NSString::new(name);
        unsafe {
            use cacao::core_graphics::geometry::{CGPoint, CGRect, CGSize};
            use cacao::foundation::NSMutableDictionary;
            let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
            let main_menu: id = msg_send![shared_app, mainMenu];
            let item_zero: id = msg_send![main_menu, itemAtIndex:0];
            let app_menu: id = msg_send![item_zero, submenu];

            let font: id = msg_send![class!(NSFont), boldSystemFontOfSize:13];
            let mut dict: NSMutableDictionary = NSMutableDictionary::new();
            // This dictionary key name needs to be corrected.
            dict.insert(NSString::new("NSFontAttributeName"), font);
            let dict_objc: id = dict.into_inner();
            let size: CGSize = msg_send![string, sizeWithAttributes:dict_objc];
            let alloc: id = msg_send![class!(NSImage), alloc];
            let image: id = msg_send![alloc, initWithSize:size];
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
    fn set_app_icon() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let image: Image = Image::with_data(include_bytes!("../../images/icon.gif"));
            unsafe {
                let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
                let _: () = msg_send![shared_app, setApplicationIconImage:image];
            }
        })
    }

    /// Starts the game.
    fn run(&mut self) {
        fastrand::seed(UNIX_EPOCH.elapsed().unwrap_or_default().as_secs());
        self.tile_variant
            .set(from_fn(|_i| from_fn(|_j| fastrand::i16(-1..i16::MAX) + 1)));
        todo!()
    }
}

#[inline]
fn fixed_size_window_config() -> WindowConfig {
    let mut config = WindowConfig::default();
    config.set_styles(&[
        WindowStyle::Titled,
        WindowStyle::Closable,
        WindowStyle::Miniaturizable,
    ]);
    config
}

struct AboutWindow {
    window: OnceAssign<Window>,

    text: Label,
}

impl AboutWindow {
    /// Create the object without `alloc` and `init` on the objc side.
    fn new() -> Self {
        Self {
            text: Default::default(),
            window: OnceAssign::new(),
        }
    }
}

impl WindowDelegate for AboutWindow {
    const NAME: &'static str = "CORAboutWindowDelegate";

    fn did_load(&mut self, window: Window) {
        self.window.set(window);
        self.window.set_content_size(390, 125);
        self.window.set_title("About");

        // Set font as Menlo.
        unsafe {
            let cls = class!(NSFont);
            let default_size: f64 = msg_send![cls, labelFontSize];
            let font_name: NSString = NSString::new("Menlo");
            let font: id = msg_send![cls, fontWithName:font_name size:default_size];
            self.text.objc.with_mut(|obj| {
                let _: () = msg_send![obj, setFont:font];
            })
        }
        self.text.set_text(concat!(
            include_str!("../../ascii-art.txt"),
            build_time_local!("%F %T %:z")
        ));

        self.window.set_content_view(&self.text);
    }
}
