use std::{
    array::from_fn,
    mem::ManuallyDrop,
    net::{SocketAddr, UdpSocket},
    thread::sleep,
    time::{Duration, Instant, UNIX_EPOCH},
};

use crate::{
    app::{
        config::ACTIVATE,
        render::{
            draw_int, draw_line, draw_str, draw_tile, draw_tile_2h, draw_tile_noise, is_cliff,
            is_within_grid, pop_to_symbol, pos_x, pos_y, time_to_ymd, Renderer, TILE_HEIGHT,
            TILE_WIDTH, TYPE_HEIGHT, TYPE_WIDTH,
        },
    },
    util::{app_from_objc, OnceAssign},
};
use build_time::build_time_local;
use cacao::{
    appkit::{
        menu::{Menu, MenuItem},
        window::{Window, WindowConfig, WindowDelegate, WindowStyle},
        App, AppDelegate, Event, EventMonitor,
    },
    color::Color,
    core_graphics::{
        base::CGFloat,
        geometry::{CGPoint, CGRect, CGSize},
    },
    events::EventModifierFlag,
    foundation::{id, nil, AutoReleasePool, NSString},
    image::Image,
    layout::Layout,
    objc::{class, msg_send, runtime::Bool},
    pasteboard::Pasteboard,
    text::Label,
    utils::sync_main_thread,
};
use curseofrust::{
    grid::{HabitLand, Tile},
    state::{MultiplayerOpts, State, UI},
    Player, Pos, Speed, FLAG_POWER, MAX_HEIGHT, MAX_PLAYERS, MAX_WIDTH,
};
use dispatch::{Queue, QueueAttribute};
use itoa::Buffer;
use local_ip_address::{local_ip, local_ipv6};
use msg::{bytemuck, server_msg, S2CData, C2S_SIZE, S2C_SIZE};

mod config;
mod render;

pub struct CorApp {
    // View-associated
    game_window: Window<GameWindow>,
    about_window: Window<AboutWindow>,
    gui_config_window: Window<config::GraphicalConfigWindow>,
    text_config_window: Window<config::TextualConfigWindow>,
    help_window: Window<HelpWindow>,
    // Game-associated
    state: Option<State>,
    tile_variant: Option<[[i16; MAX_HEIGHT as usize]; MAX_WIDTH as usize]>,
    pop_variant: Option<[[i16; MAX_HEIGHT as usize]; MAX_WIDTH as usize]>,
    ui: Option<UI>,
    // Misc
    queue: Queue,
    _listener: EventMonitor,
    /// Indicates whether:
    /// - a local game has already started.
    /// - the client has successfully connected to a server.
    run: bool,
    /// Should terminate game and switch back to error message view.
    terminate: bool,
    /// [`Some`] if playing a multiplayer game.
    socket: Option<UdpSocket>,
}

impl AppDelegate for CorApp {
    fn did_finish_launching(&self) {
        self.game_window.show();
        App::set_menu(Self::menu());
        // Self::change_app_menu_name("CoR");
        App::activate();
        // Self::set_app_icon();
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
            game_window: Window::with(fixed_size_window_config(), GameWindow::new()),
            about_window: Window::with(fixed_size_window_config(), AboutWindow::new()),
            gui_config_window: Window::with(
                fixed_size_window_config(),
                config::GraphicalConfigWindow::new(),
            ),
            text_config_window: Window::with(
                fixed_size_window_config(),
                config::TextualConfigWindow::new(),
            ),
            help_window: Window::with(fixed_size_window_config(), HelpWindow::new()),
            state: None,
            tile_variant: None,
            pop_variant: None,
            ui: None,
            queue: Queue::create(
                "com.dm.earth.curseofrust.worker",
                QueueAttribute::Concurrent,
            ),
            _listener: Event::local_monitor(cacao::appkit::EventMask::KeyDown, |e| {
                let app = app_from_objc::<Self>();
                if app.run && app.game_window.is_key() {
                    let keycode: u16 = unsafe { msg_send![&e.0, keyCode] };
                    app.queue
                        .exec_sync(|| !app_from_objc::<Self>().process_input(keycode))
                        .then_some(e)
                } else {
                    Some(e)
                }
            }),
            run: false,
            terminate: false,
            socket: None,
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
            let _: () = unsafe { msg_send![&obj, setEnabled:Bool::NO] };
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
            .action(|| {
                let this = app_from_objc::<Self>();
                if !this.run {
                    this.queue.exec_async(|| app_from_objc::<Self>().pre_run())
                }
            });
        let help = MenuItem::new("Curse of Rust Help").action(|| {
            let app = app_from_objc::<Self>();
            app.help_window.show();
        });
        let main_menu = Menu::new(
            "CoR Cocoa",
            vec![
                about,
                MenuItem::Separator,
                preferences,
                MenuItem::Separator,
                MenuItem::Hide,
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
        let help_menu = Menu::new("Help", vec![help]);
        // Required for disabling menu items.
        let _: () = unsafe { msg_send![&file_menu.0, setAutoenablesItems:Bool::NO] };
        vec![main_menu, file_menu, help_menu]
    }

    /*
        /// Loses main menu's bold style.
        fn _change_app_menu_name(name: &str) {
            let pool = ManuallyDrop::new(AutoReleasePool::new());
            let string = NSString::new(name);
            unsafe {
                let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
                let main_menu: id = msg_send![shared_app, mainMenu];
                let item_zero: id = msg_send![main_menu, itemAtIndex:0];
                let app_menu: id = msg_send![item_zero, submenu];
                let _: () = msg_send![app_menu, setTitle:string.objc.autorelease_return()];
            }
            pool.drain();
        }

        /// Very raw, very ugly.
        fn _draw_and_set_app_menu_name(name: &str) {
            let pool = ManuallyDrop::new(AutoReleasePool::new());
            let string: NSString = NSString::new(name);
            unsafe {
                use cacao::foundation::NSMutableDictionary;
                let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
                let main_menu: id = msg_send![shared_app, mainMenu];
                let item_zero: id = msg_send![main_menu, itemAtIndex:0];
                let app_menu: id = msg_send![item_zero, submenu];

                let font: id = msg_send![class!(NSFont), boldSystemFontOfSize:13];
                let mut dict: NSMutableDictionary = NSMutableDictionary::new();
                // This dictionary key name needs to be corrected.
                dict.insert(NSString::new("NSFontAttributeName"), font);
                let dict_objc = dict.0.autorelease_return();
                let size: CGSize = msg_send![&string.objc, sizeWithAttributes:dict_objc];
                let alloc: id = msg_send![class!(NSImage), alloc];
                let image: id = msg_send![alloc, initWithSize:size];
                let _: () = msg_send![image, lockFocus];
                let rect: CGRect = CGRect::new(&CGPoint::new(0.0, 0.5), &size);
                let _: () = msg_send![&string.objc, drawWithRect:rect options:1<<0 attributes:dict_objc context:nil];
                let _: () = msg_send![image, unlockFocus];

                let _: () = msg_send![app_menu, setTitle:NSString::new("").objc.autorelease_return()];
                let _: () = msg_send![item_zero, setImage:image];
            }
            pool.drain();
        }

        /// Icon is hard-coded, so call this only once.\
        /// Just modify this fn if you want to change icon.
        fn _set_app_icon() {
            static ONCE: Once = Once::new();
            ONCE.call_once(|| {
                let pool = ManuallyDrop::new(AutoReleasePool::new());
                let image: Image = Image::with_data(include_bytes!("../../images/icon.gif"));
                unsafe {
                    let shared_app: id = msg_send![class!(RSTApplication), sharedApplication];
                    let _: () =
                        msg_send![shared_app, setApplicationIconImage:image.0.autorelease_return()];
                }
                pool.drain();
            })
        }
    */

    /// Starts the game.
    fn pre_run(&mut self) {
        sync_main_thread(|| {
            app_from_objc::<Self>().game_window.show();
        });
        fastrand::seed(UNIX_EPOCH.elapsed().unwrap_or_default().as_secs());
        match self.load_config() {
            Ok(cli_parser::Options {
                basic, multiplayer, ..
            }) => {
                let common_init = || {
                    match State::new(basic) {
                        Ok(state) => self.state = Some(state),
                        Err(err) => {
                            self.game_window
                                .delegate
                                .as_ref()
                                .unwrap()
                                .display_err(&err.to_string(), None);
                            return false;
                        }
                    };
                    self.tile_variant = Some(from_fn(|_i| from_fn(|_j| fastrand::i16(..))));
                    self.pop_variant = Some(from_fn(|_i| from_fn(|_j| fastrand::i16(..))));
                    self.ui = Some(UI::new(self.state.as_ref().unwrap()));
                    true
                };
                match multiplayer {
                    MultiplayerOpts::None => {
                        if !common_init() {
                            return;
                        }
                        self.run();
                    }
                    MultiplayerOpts::Server { .. } => {
                        self.game_window.delegate.as_ref().unwrap().display_err(
                            "Integrated server is currently not implemented, please use dedicated server.",
                            Some(Color::SystemOrange),
                        );
                    }
                    MultiplayerOpts::Client { .. } => {
                        self.game_window.delegate.as_ref().unwrap().display_err(
                            "UDP multiplayer client is not usable, please use the console version.",
                            Some(Color::SystemOrange),
                        );
                        /* if !common_init() {
                            return;
                        }
                        if let Err((msg, color)) = self.run_client(server, port) {
                            self.game_window
                                .delegate
                                .as_ref()
                                .unwrap()
                                .display_err(&msg, color);
                        } */
                    }
                }
            }
            Err(err) => {
                self.game_window
                    .delegate
                    .as_ref()
                    .unwrap()
                    .display_err(&err.to_string(), None);
            }
        }
    }

    /// Start a singleplayer game.
    fn run(&mut self) {
        self.run = true;
        let seed = self.state.as_ref().unwrap().seed;
        sync_main_thread(move || {
            let this = app_from_objc::<Self>();
            this.game_window
                .set_title(format!("Singleplayer - seed: {}", seed).as_str());
            // Set content view
            this.game_window
                .set_content_view(this.game_window.delegate.as_ref().unwrap().renderer.view());
        });
        let old_frame = self.init_screen();
        let mut prev_time = Instant::now();
        let mut k: u16 = 0;
        let mut itoa_buf = Buffer::new();
        while !self.terminate {
            if Instant::now().duration_since(prev_time) >= DELAY {
                prev_time += DELAY;
                k += 1;
                if k >= 1600 {
                    k = 0;
                }
                if k % slowdown(self.state.as_ref().unwrap().speed) == 0
                    && self.state.as_ref().unwrap().speed != Speed::Pause
                {
                    let state = self.state.as_mut().unwrap();
                    state.kings_move();
                    state.simulate();
                }
                if k % 5 == 0 {
                    self.render(&mut itoa_buf);
                }
            } else {
                sleep(DELAY / 2);
            }
        }
        sync_main_thread(move || {
            let this = app_from_objc::<Self>();
            let _: () = unsafe {
                msg_send![&this.game_window.objc, setFrame:old_frame display:Bool::YES animate:Bool::YES]
            };
        });
        self.game_window.delegate.as_ref().unwrap().restore(false);
        // Finalize game view
        self.game_window
            .delegate
            .as_mut()
            .unwrap()
            .renderer
            .finalize_renderer();
        self.terminate = false;
        self.run = false;
    }

    /// Start as a multiplayer client.
    fn _run_client(
        &mut self,
        server: SocketAddr,
        port: u16,
    ) -> Result<(), (String, Option<Color>)> {
        let mut prev_time = Instant::now();
        let mut k: u16 = 0;
        let local_addr = SocketAddr::new(
            match server {
                SocketAddr::V4(_) => local_ip(),
                SocketAddr::V6(_) => local_ipv6(),
            }
            .map_err(|e| ("local_ip error: ".to_owned() + &e.to_string(), None))?,
            port,
        );
        let socket = UdpSocket::bind(local_addr)
            .map_err(|e| ("bind error: ".to_owned() + &e.to_string(), None))?;
        socket
            .connect(server)
            .map_err(|e| ("connect error: ".to_owned() + &e.to_string(), None))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| ("set_nonblocking error: ".to_owned() + &e.to_string(), None))?;
        self.socket = Some(socket);
        let mut s2c_buf = [0u8; S2C_SIZE];
        let mut old_frame: CGRect = Default::default();
        let mut itoa_buf = Buffer::new();
        while !self.terminate {
            if Instant::now().duration_since(prev_time) >= DELAY {
                prev_time += DELAY;
                k += 1;
                k %= 1600;

                if k % 50 == 0 {
                    const ALIVE_PACKET: [u8; C2S_SIZE] = [msg::client_msg::IS_ALIVE, 0, 0, 0];
                    self.socket
                        .as_ref()
                        .unwrap()
                        .send(&ALIVE_PACKET)
                        .map_err(|e| ("send error: ".to_owned() + &e.to_string(), None))?;
                }

                // Start fetch state
                let socket_ref = self.socket.as_ref().unwrap();
                let nread = socket_ref
                    .recv(&mut s2c_buf)
                    .map_err(|e| ("recv error: ".to_owned() + &e.to_string(), None))?;
                if nread < S2C_SIZE {
                    Err((format!("short read: {}<{}", nread, S2C_SIZE), None))?;
                }
                let (&msg, body) = s2c_buf
                    .split_first()
                    .expect("s2c_buf should be longer than one byte");
                let data: S2CData = *bytemuck::from_bytes(body);
                if msg == server_msg::STATE {
                    msg::apply_s2c_msg(self.state.as_mut().unwrap(), data)
                        .map_err(|e| ("apply_s2c_msg error: ".to_owned() + &e.to_string(), None))?;
                    if !self.run {
                        self.run = true;
                        old_frame = self.init_screen();
                        self.ui = Some(UI::new(self.state.as_ref().unwrap()));
                    }
                }
                // End fetch state

                if self.run && k % 5 == 0 {
                    self.render(&mut itoa_buf);
                }
            } else {
                sleep(DELAY / 2);
            }
        }
        // Clean up.
        sync_main_thread(move || {
            let this = app_from_objc::<Self>();
            let _: () = unsafe {
                msg_send![&this.game_window.objc, setFrame:old_frame display:Bool::YES animate:Bool::YES]
            };
        });
        self.game_window.delegate.as_ref().unwrap().restore(false);
        self.run = false;
        self.terminate = false;
        // Drop UdpSocket.
        self.socket = None;
        Ok(())
    }

    pub fn load_config(&self) -> Result<cli_parser::Options, cli_parser::Error> {
        let mut config_str = self
            .text_config_window
            .delegate
            .as_ref()
            .unwrap()
            .input
            .get_value()
            .trim()
            .to_owned();
        if config_str.starts_with('-') {
            // Add fake bin name.
            config_str = "curseofrust ".to_owned() + &config_str;
        }
        config_str = config_str.replace("-v", "").replace("-h", "");
        cli_parser::parse_to_options(config_str.split_whitespace())
    }

    /// Returns `true` if the event is consumed.
    fn process_input(&mut self, carbon_keycode: u16) -> bool {
        // Move cursor
        const K_LEFT: u16 = 0x7B;
        const K_RIGHT: u16 = 0x7C;
        const K_DOWN: u16 = 0x7D;
        const K_UP: u16 = 0x7E;
        // Another move cursor
        /// Move cursor left.
        const K_H: u16 = 0x04;
        /// Move cursor down.
        const K_J: u16 = 0x26;
        /// Move cursor up.
        const K_K: u16 = 0x28;
        /// Move cursor right.
        const K_L: u16 = 0x25;
        /// Quit.
        const K_Q: u16 = 0x0C;
        /// Flag.
        const K_SPACE: u16 = 0x31;
        /// Slower.
        const K_S: u16 = 0x01;
        /// Faster.
        const K_F: u16 = 0x03;
        /// Pause game.
        const K_P: u16 = 0x23;
        /// Build.
        const K_R: u16 = 0x0F;
        /// Another build.
        const K_V: u16 = 0x09;
        /// Remove all flags.
        const K_X: u16 = 0x07;
        /// Remove half flags.
        const K_C: u16 = 0x08;

        macro_rules! c2s_msg {
            ($msg:ident, $info:expr) => {{
                let data: msg::C2SData = (self.ui.as_ref().unwrap().cursor, $info).into();
                let mut buf = [0u8; C2S_SIZE];
                let (msg, d) = buf
                    .split_first_mut()
                    .expect("the buffer should longer than one byte");
                *msg = msg::client_msg::$msg;
                d.copy_from_slice(bytemuck::bytes_of(&data));
                let socket = self.socket.as_ref().unwrap();
                let _ = socket.send(&buf);
            }};
            ($msg:ident) => {
                c2s_msg!($msg, 0)
            };
        }

        let multiplayer = self.socket.is_some();

        match carbon_keycode {
            K_LEFT | K_H => {
                let ui = self.ui.as_mut().unwrap();
                let mut cursor = ui.cursor;
                cursor.0 -= 1;
                ui.adjust_cursor(self.state.as_ref().unwrap(), cursor);
            }
            K_RIGHT | K_L => {
                let ui = self.ui.as_mut().unwrap();
                let mut cursor = ui.cursor;
                cursor.0 += 1;
                ui.adjust_cursor(self.state.as_ref().unwrap(), cursor);
            }
            K_UP | K_K => {
                let ui = self.ui.as_mut().unwrap();
                let mut cursor = ui.cursor;
                cursor.1 -= 1;
                if cursor.1 % 2 == 1 {
                    cursor.0 += 1;
                }
                ui.adjust_cursor(self.state.as_ref().unwrap(), cursor);
            }
            K_DOWN | K_J => {
                let ui = self.ui.as_mut().unwrap();
                let mut cursor = ui.cursor;
                cursor.1 += 1;
                if cursor.1 % 2 == 0 {
                    cursor.0 -= 1;
                }
                ui.adjust_cursor(self.state.as_ref().unwrap(), cursor);
            }
            K_SPACE => {
                let state = self.state.as_mut().unwrap();
                let fg = &mut state.fgs[state.controlled.0 as usize];
                let cursor = self.ui.as_ref().unwrap().cursor;
                if !multiplayer {
                    if fg.is_flagged(cursor) {
                        fg.remove(&state.grid, cursor, FLAG_POWER);
                    } else {
                        fg.add(&state.grid, cursor, FLAG_POWER);
                    }
                } else if fg.is_flagged(cursor) {
                    c2s_msg!(FLAG_OFF);
                } else {
                    c2s_msg!(FLAG_ON);
                }
            }
            K_Q => self.terminate = true,
            K_S => {
                let speed = &mut self.state.as_mut().unwrap().speed;
                *speed = speed.slower();
            }
            K_F => {
                let speed = &mut self.state.as_mut().unwrap().speed;
                *speed = speed.faster();
            }
            K_P => {
                let state = self.state.as_mut().unwrap();
                let speed = &mut state.speed;
                let prev_speed = &mut state.prev_speed;
                if !multiplayer {
                    if *speed == Speed::Pause {
                        *speed = *prev_speed;
                    } else {
                        *prev_speed = *speed;
                        *speed = Speed::Pause;
                    }
                } else if *speed == Speed::Pause {
                    c2s_msg!(UNPAUSE);
                } else {
                    *prev_speed = *speed;
                    c2s_msg!(PAUSE);
                }
            }
            K_R | K_V => {
                if !multiplayer {
                    let state = self.state.as_mut().unwrap();
                    let _ = state.grid.build(
                        &mut state.countries[state.controlled.0 as usize],
                        self.ui.as_ref().unwrap().cursor,
                    );
                } else {
                    c2s_msg!(BUILD);
                }
            }
            K_X => {
                if !multiplayer {
                    let state = self.state.as_mut().unwrap();
                    state.fgs[state.controlled.0 as usize].remove_with_prob(&state.grid, 1.);
                } else {
                    c2s_msg!(FLAG_OFF_ALL);
                }
            }
            K_C => {
                if !multiplayer {
                    let state = self.state.as_mut().unwrap();
                    state.fgs[state.controlled.0 as usize].remove_with_prob(&state.grid, 0.5);
                } else {
                    c2s_msg!(FLAG_OFF_HALF);
                }
            }
            _ => return false,
        }
        true
    }

    /// Render the current [`State`].
    fn render(&mut self, itoa_buf: &mut Buffer) {
        let pool = ManuallyDrop::new(AutoReleasePool::new());
        // Render start.
        self.game_window
            .delegate
            .as_ref()
            .unwrap()
            .renderer
            .init_frame();
        let state = self.state.as_ref().unwrap();
        let ui = self.ui.as_ref().unwrap();
        let tile_var = self.tile_variant.as_ref().unwrap();
        for j in 0..state.grid.height() as i16 {
            for i in -1..state.grid.width() as i16 + 1 {
                // Draw cliffs.
                let cliff = is_cliff(i, j, &state.grid);
                if cliff.contains(&true) {
                    for (idx, bl) in cliff.iter().enumerate() {
                        if *bl {
                            draw_tile(7 + idx as i16, 0, pos_x(ui, i), pos_y(j));
                        }
                    }
                    continue;
                }
                if !is_within_grid(i, j, &state.grid) {
                    continue;
                }
                match state.grid.tile(Pos(i as i32, j as i32)).unwrap() {
                    Tile::Habitable { land, units, owner } => {
                        // Draw grass.
                        draw_tile(
                            (tile_var[i as usize][j as usize] % 6).abs(),
                            (tile_var[i as usize][j as usize] / 6 % 3).abs(),
                            pos_x(ui, i),
                            pos_y(j),
                        );
                        match land {
                            HabitLand::Village => {
                                draw_tile_2h(0, 7 + 3 * owner.0 as i16, pos_x(ui, i), pos_y(j))
                            }
                            HabitLand::Town => {
                                draw_tile_2h(1, 7 + 3 * owner.0 as i16, pos_x(ui, i), pos_y(j))
                            }
                            HabitLand::Fortress => {
                                draw_tile_2h(2, 7 + 3 * owner.0 as i16, pos_x(ui, i), pos_y(j))
                            }
                            _ => {
                                let pop = units[owner.0 as usize];
                                if pop > 0 {
                                    draw_tile_noise(
                                        pop_to_symbol(pop),
                                        8 + 3 * owner.0 as i16,
                                        pos_x(ui, i),
                                        pos_y(j),
                                        self.pop_variant.as_ref().unwrap()[i as usize][j as usize],
                                    );
                                    if fastrand::i16(..) % 20 == 0 {
                                        let mut d = 1_i16;
                                        if owner != &state.controlled {
                                            d += 10;
                                        }
                                        let old_var = self.pop_variant.as_ref().unwrap()
                                            [i as usize][j as usize];
                                        self.pop_variant.as_mut().unwrap()[i as usize]
                                            [j as usize] = (old_var + d) % 10000;
                                    }
                                }
                            }
                        }
                    }
                    Tile::Mine(owner) => {
                        // Draw grass.
                        draw_tile(
                            (tile_var[i as usize][j as usize] % 6).abs(),
                            (tile_var[i as usize][j as usize] / 6 % 3).abs(),
                            pos_x(ui, i),
                            pos_y(j),
                        );
                        // Draw mountain.
                        draw_tile_2h(
                            (tile_var[i as usize][j as usize] % 5).abs(),
                            5,
                            pos_x(ui, i),
                            pos_y(j),
                        );
                        // Draw mine.
                        if owner.is_neutral() {
                            draw_tile(5, 5, pos_x(ui, i), pos_y(j));
                        } else {
                            // Draw currency sign if controlled by a player.
                            draw_tile_2h(5, 5, pos_x(ui, i), pos_y(j));
                        }
                    }
                    Tile::Mountain => {
                        // Draw grass.
                        draw_tile(
                            (tile_var[i as usize][j as usize] % 6).abs(),
                            (tile_var[i as usize][j as usize] / 6 % 3).abs(),
                            pos_x(ui, i),
                            pos_y(j),
                        );
                        // Draw mountain.
                        draw_tile_2h(
                            (tile_var[i as usize][j as usize] % 5).abs(),
                            5,
                            pos_x(ui, i),
                            pos_y(j),
                        );
                    }
                    _ => {}
                }
                // Draw flags.
                for p in 0..MAX_PLAYERS as u32 {
                    if state.fgs[p as usize].is_flagged(Pos(i as i32, j as i32)) {
                        draw_tile_2h(
                            match Player(p) == state.controlled {
                                true => 3,
                                false => 4,
                            },
                            7 + 3 * p as i16,
                            pos_x(ui, i),
                            pos_y(j),
                        );
                    }
                }
            }
        }
        // Draw cursor.
        draw_tile_2h(
            6,
            5,
            pos_x(ui, ui.cursor.0 as i16 - 1),
            pos_y(ui.cursor.1 as i16),
        );
        draw_tile_2h(
            7,
            5,
            pos_x(ui, ui.cursor.0 as i16),
            pos_y(ui.cursor.1 as i16),
        );
        draw_tile_2h(
            8,
            5,
            pos_x(ui, ui.cursor.0 as i16 + 1),
            pos_y(ui.cursor.1 as i16),
        );
        // Draw text.
        let base_y = (pos_y(state.grid.height() as i16) + 1) * TILE_HEIGHT;
        draw_str("Gold:", Player::NEUTRAL, TILE_WIDTH, base_y);
        draw_int(
            state.countries[state.controlled.0 as usize].gold,
            state.controlled,
            TILE_WIDTH + 6 * TYPE_WIDTH,
            base_y,
            itoa_buf,
        );
        draw_str(
            "Prices: 160 240 320",
            Player::NEUTRAL,
            TILE_WIDTH,
            base_y + TYPE_HEIGHT,
        );
        draw_str(
            "Date:",
            Player::NEUTRAL,
            TILE_WIDTH + 54 * TYPE_WIDTH,
            base_y,
        );
        let (y, m, d) = time_to_ymd(state.time);
        draw_int(
            y,
            state.controlled,
            TILE_WIDTH + 60 * TYPE_WIDTH,
            base_y,
            itoa_buf,
        );
        draw_str("-", state.controlled, TILE_WIDTH + 64 * TYPE_WIDTH, base_y);
        if m > 9 {
            draw_int(
                m,
                state.controlled,
                TILE_WIDTH + 65 * TYPE_WIDTH,
                base_y,
                itoa_buf,
            );
        } else {
            draw_str("0", state.controlled, TILE_WIDTH + 65 * TYPE_WIDTH, base_y);
            draw_int(
                m,
                state.controlled,
                TILE_WIDTH + 66 * TYPE_WIDTH,
                base_y,
                itoa_buf,
            );
        }
        draw_str("-", state.controlled, TILE_WIDTH + 67 * TYPE_WIDTH, base_y);
        if d > 9 {
            draw_int(
                d,
                state.controlled,
                TILE_WIDTH + 68 * TYPE_WIDTH,
                base_y,
                itoa_buf,
            );
        } else {
            draw_str("0", state.controlled, TILE_WIDTH + 68 * TYPE_WIDTH, base_y);
            draw_int(
                d,
                state.controlled,
                TILE_WIDTH + 69 * TYPE_WIDTH,
                base_y,
                itoa_buf,
            );
        }
        draw_str(
            "Speed:",
            Player::NEUTRAL,
            TILE_WIDTH + 54 * TYPE_WIDTH,
            base_y + TYPE_HEIGHT,
        );
        draw_str(
            match state.speed {
                Speed::Fast => "Fast",
                Speed::Faster => "Faster",
                Speed::Fastest => "Fastest",
                Speed::Normal => "Normal",
                Speed::Pause => "Pause",
                Speed::Slow => "Slow",
                Speed::Slower => "Slower",
                Speed::Slowest => "Slowest",
            },
            Player::NEUTRAL,
            TILE_WIDTH + 61 * TYPE_WIDTH,
            base_y + TYPE_HEIGHT,
        );
        draw_str(
            "Population:",
            Player::NEUTRAL,
            TILE_WIDTH + 23 * TYPE_WIDTH,
            base_y,
        );
        for p in 1..MAX_PLAYERS {
            let pop_str = itoa_buf.format(
                state
                    .grid
                    .tile(Pos(ui.cursor.0, ui.cursor.1))
                    .unwrap()
                    .units()[p],
            );
            let offset = 3 - pop_str.len();
            draw_str(
                pop_str,
                Player(p as u32),
                TILE_WIDTH + (23 + 4 * (p as i16 - 1)) * TYPE_WIDTH + offset as i16,
                base_y + TYPE_HEIGHT,
            );
        }
        draw_str(
            "[Space] flag",
            Player::NEUTRAL,
            TILE_WIDTH,
            base_y + 3 * TYPE_HEIGHT,
        );
        draw_str(
            "[R] or [V] build",
            Player::NEUTRAL,
            TILE_WIDTH + 27 * TYPE_WIDTH,
            base_y + 3 * TYPE_HEIGHT,
        );
        draw_str(
            "[X],[C] mass remove",
            Player::NEUTRAL,
            TILE_WIDTH,
            base_y + 4 * TYPE_HEIGHT,
        );
        draw_str(
            "[S] slower [F] faster",
            Player::NEUTRAL,
            TILE_WIDTH + 54 * TYPE_WIDTH,
            base_y + 3 * TYPE_HEIGHT,
        );
        draw_str(
            "[P] pause",
            Player::NEUTRAL,
            TILE_WIDTH + 54 * TYPE_WIDTH,
            base_y + 4 * TYPE_HEIGHT,
        );
        // Draw line.
        draw_line(base_y);
        // Finalize frame rendering
        self.game_window
            .delegate
            .as_ref()
            .unwrap()
            .renderer
            .finalize_frame();

        // Flush.
        sync_main_thread(|| {
            app_from_objc::<Self>()
                .game_window
                .delegate
                .as_ref()
                .unwrap()
                .renderer
                .view()
                .set_needs_display(true);
        });

        pool.drain();
    }

    /// Returns `old_frame`.
    fn init_screen(&mut self) -> CGRect {
        let screen_size = CGSize::new(
            i16::max(
                (self.ui.as_ref().unwrap().xlen + 2) as i16 * TILE_WIDTH,
                75 * TYPE_WIDTH + TILE_WIDTH,
            )
            .into(),
            ((self.state.as_ref().unwrap().grid.height() as u16 + 3) as i16 * TILE_HEIGHT
                + 5 * TYPE_HEIGHT)
                .into(),
        );
        let old_frame: CGRect;
        unsafe {
            // Resize window to fit `screen`.
            old_frame = msg_send![&self.game_window.objc, frame];
            let old_content: CGRect =
                msg_send![&self.game_window.objc, contentRectForFrameRect:old_frame];
            let new_content = CGRect::new(
                &CGPoint::new(
                    old_content.origin.x,
                    old_content.origin.y + old_content.size.height - screen_size.height,
                ),
                &screen_size,
            );
            let new_frame: CGRect =
                msg_send![&self.game_window.objc, frameRectForContentRect:new_content];
            sync_main_thread(move || {
                let this = app_from_objc::<Self>();
                let _: () = msg_send![&this.game_window.objc, setFrame:new_frame display:Bool::YES animate:Bool::YES];
            });
        }
        self.game_window
            .delegate
            .as_mut()
            .unwrap()
            .renderer
            .init_renderer(screen_size);
        old_frame
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

        set_font(&self.text, "Menlo", None);

        self.text.set_text(concat!(
            include_str!("../../ascii-art.txt"),
            build_time_local!("%F %T %:z")
        ));

        self.window.set_content_view(&self.text);
    }
}

/// Set font as `name`.
fn set_font(obj: &Label, name: &str, size: Option<f64>) {
    let pool = ManuallyDrop::new(AutoReleasePool::new());
    unsafe {
        let cls = class!(NSFont);
        let size: f64 = size.unwrap_or_else(|| msg_send![cls, labelFontSize]);
        let font_name: NSString = NSString::new(name);
        let font: id = msg_send![cls, fontWithName:font_name.objc.autorelease_return() size:size];
        obj.objc.with_mut(|obj| {
            let _: () = msg_send![obj, setFont:font];
        })
    }
    pool.drain();
}

struct HelpWindow {
    window: OnceAssign<Window>,

    text: Label,
}

impl HelpWindow {
    fn new() -> Self {
        Self {
            window: OnceAssign::new(),
            text: Label::new(),
        }
    }
}

impl WindowDelegate for HelpWindow {
    const NAME: &'static str = "CORHelpWindowDelegate";

    fn did_load(&mut self, window: Window) {
        self.window.set(window);
        self.window.set_content_size(390, 600);
        self.window.set_title("Help");
        self.text.set_text(cli_parser::HELP_MSG);
        set_font(&self.text, "Menlo", Some(8.));
        self.window.set_content_view(&self.text);
    }
}

struct GameWindow {
    window: OnceAssign<Window>,

    err_msg: Label,
    renderer: Renderer,
}

impl GameWindow {
    fn new() -> Self {
        Self {
            window: OnceAssign::new(),
            err_msg: Label::new(),
            renderer: Renderer::new(),
        }
    }

    fn display_err(&self, msg: &str, color: Option<Color>) {
        self.window.set_title("corCocoa");
        self.err_msg.set_text(msg);
        self.err_msg
            .set_text_color(color.unwrap_or(Color::SystemRed));
        self.window.set_content_view(&self.err_msg);
        self.resize_window(200, 150);
    }

    /// Set the window to initial state.
    fn restore(&self, resize: bool) {
        let main: Bool = unsafe { msg_send![class!(NSThread), isMainThread] };
        if main.as_bool() {
            self.window.set_title("corCocoa");
        } else {
            sync_main_thread(|| app_from_objc::<CorApp>().game_window.set_title("corCocoa"))
        }

        self.err_msg.set_text_color(Color::Label);
        self.err_msg
            .set_text("Preference parsing error will be emitted here.");
        if main.as_bool() {
            self.window.set_content_view(&self.err_msg);
        } else {
            sync_main_thread(|| {
                let app = app_from_objc::<CorApp>();
                app.game_window
                    .set_content_view(&app.game_window.delegate.as_ref().unwrap().err_msg);
            })
        }
        if resize {
            self.resize_window(200, 150);
        }
    }

    fn resize_window<F>(&self, width: F, height: F)
    where
        F: Into<CGFloat>,
    {
        let mut frame: CGRect = unsafe { msg_send![&self.window.objc, frame] };
        frame.size = CGSize::new(width.into(), height.into());
        sync_main_thread(move || {
            let _: () = unsafe {
                msg_send![&app_from_objc::<CorApp>().game_window.objc, setFrame:frame display:Bool::YES animate:Bool::YES]
            };
        })
    }
}

impl WindowDelegate for GameWindow {
    const NAME: &'static str = "CORGameWindowDelegate";

    fn did_load(&mut self, window: Window) {
        self.window.set(window);
        self.window.set_content_size(200, 150);
        self.restore(false);
    }
}

/// 10 ms.
const DELAY: Duration = Duration::from_nanos(10_000_000);

#[inline]
fn slowdown(speed: Speed) -> u16 {
    match speed {
        // Will never be used.
        Speed::Pause => u16::MAX,
        Speed::Slowest => 160,
        Speed::Slower => 80,
        Speed::Slow => 40,
        Speed::Normal => 20,
        Speed::Fast => 10,
        Speed::Faster => 5,
        Speed::Fastest => 2,
    }
}
