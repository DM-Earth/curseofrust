use std::array::from_fn;
use std::sync::Once;
use std::time::UNIX_EPOCH;

use crate::{
    app::config::ACTIVATE,
    util::{app_from_objc, OnceAssign},
};
use build_time::build_time_local;
use cacao::{
    appkit::{
        menu::{Menu, MenuItem},
        window::{Window, WindowConfig, WindowDelegate, WindowStyle},
        App, AppDelegate,
    },
    events::EventModifierFlag,
    foundation::{id, nil, NSString},
    image::Image,
    objc::{class, msg_send, sel, sel_impl},
    pasteboard::Pasteboard,
    text::Label,
};
use curseofrust::state::{BasicOpts, MultiplayerOpts, State};
use curseofrust::{MAX_HEIGHT, MAX_WIDTH};

use self::config::TextualConfigWindow;

mod config;

pub struct CorApp {
    // View-associated
    game_window: Window,
    about_window: Window<AboutWindow>,
    gui_config_window: Window<config::GraphicalConfigWindow>,
    text_config_window: Window<config::TextualConfigWindow>,
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

    fn should_handle_reopen(&self, has_visible_windows: bool) -> bool {
        if has_visible_windows {
            false
        } else {
            self.game_window.show();
            true
        }
    }
}

impl CorApp {
    pub fn new() -> Self {
        Self {
            game_window: Default::default(),
            about_window: Window::with(fixed_size_window_config(), AboutWindow::new()),
            gui_config_window: Window::with(
                fixed_size_window_config(),
                config::GraphicalConfigWindow::new(),
            ),
            text_config_window: Window::with(
                fixed_size_window_config(),
                TextualConfigWindow::new(),
            ),
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
                app.text_config_window.show();
            });
        let mut copy_config = MenuItem::new("Copy Preferences")
            .modifiers(&[EventModifierFlag::Command])
            .key("c")
            .action(|| {
                let app = app_from_objc::<Self>();
                // Not planning to use `NSUserDefaults` because I don't want anything persisted.
                // @BUG: It does nothing.
                let pb = Pasteboard::default();
                pb.copy_text(
                    app.text_config_window
                        .delegate
                        .as_ref()
                        .unwrap()
                        .input
                        .get_value(),
                );
            });
        // Disable `Copy Preferences` menu as it's not usable.
        if let MenuItem::Custom(obj) = copy_config {
            let _: () = unsafe { msg_send![obj, setEnabled:cacao::foundation::NO] };
            copy_config = MenuItem::Custom(obj);
        }
        let restore_default_config = MenuItem::new("Restore Default Preferences").action(|| {
            let app = app_from_objc::<Self>();
            if app.text_config_window.is_key() {
                app.text_config_window
                    .delegate
                    .as_ref()
                    .unwrap()
                    .input
                    .set_text(match fastrand::u8(1..(36 + 1)) {
                        // In case I forgot.
                        36 => ACTIVATE,
                        _ => "-i4 -q1 -dee -W16 -H16",
                    });
            }
        });
        let new_game = MenuItem::new("New Game")
            .modifiers(&[EventModifierFlag::Command])
            .key("n")
            .action(|| app_from_objc::<Self>().run());
        let main_menu = Menu::new(
            "CoR Cocoa",
            vec![
                about,
                MenuItem::Separator,
                preferences,
                MenuItem::Separator,
                MenuItem::Quit,
            ],
        );
        let file_menu = Menu::new(
            "File",
            vec![
                new_game,
                MenuItem::Separator,
                MenuItem::CloseWindow,
                MenuItem::Separator,
                copy_config,
                restore_default_config,
            ],
        );
        // Required for disabling menu items.
        let _: () = unsafe { msg_send![file_menu.0, setAutoenablesItems:cacao::foundation::NO] };
        vec![main_menu, file_menu]
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

    pub fn load_config(&self) -> Result<(BasicOpts, MultiplayerOpts), cli_parser::Error> {
        let mut config_str = self
            .text_config_window
            .delegate
            .as_ref()
            .unwrap()
            .input
            .get_value()
            .trim()
            .to_owned();
        if config_str.starts_with("-") {
            // Add fake bin name.
            config_str = "curseofrust ".to_owned() + &config_str;
        }
        config_str = config_str.replace("-v", "").replace("-h", "");
        cli_parser::parse(config_str.split_whitespace())
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
