use cacao::{
    appkit::{
        window::{Window, WindowDelegate},
        FocusRingType,
    },
    color::Color,
    foundation::NSUInteger,
    input::TextField,
    layout::{Layout, LayoutConstraint},
    listview::{ListView, ListViewDelegate, ListViewRow},
    objc::{msg_send, sel, sel_impl},
    text::Label,
};

use crate::util::{app_from_objc, OnceAssign};

use super::CorApp;

pub const ACTIVATE: &str = "activate gui config window c191239 5444";

/// Uses `preferences` as a replacement of `config` to fit the
/// Mac OS X language style.
pub struct GraphicalConfigWindow {
    window: OnceAssign<Window>,

    pub list: ListView<ConfigList>,
}

impl GraphicalConfigWindow {
    pub fn new() -> Self {
        Self {
            window: OnceAssign::new(),
            list: ListView::with(ConfigList::new()),
        }
    }
}

impl WindowDelegate for GraphicalConfigWindow {
    const NAME: &'static str = "CORGraphicalConfigWindowDelegate";

    fn did_load(&mut self, window: Window) {
        self.window.set(window);
        self.window.set_content_size(300, 200);
        self.window.set_title("GUI Preferences");
        self.window.set_content_view(&self.list);
    }
}

/// List containing config items in the graphical config window.\
/// Unfinished, but i will be back. **(2024-02-06 C191239)**
pub struct ConfigList {
    list: OnceAssign<ListView>,
}

impl ConfigList {
    fn new() -> Self {
        Self {
            list: OnceAssign::new(),
        }
    }
}

impl ListViewDelegate for ConfigList {
    const NAME: &'static str = "CORConfigListViewDelegate";

    fn did_load(&mut self, view: ListView) {
        self.list.set(view);
    }

    fn number_of_items(&self) -> usize {
        3
    }

    fn item_for(&self, row: usize) -> ListViewRow {
        let msg = match row {
            0 => ("Not", Color::SystemRed),
            1 => ("Implmented", Color::SystemGreen),
            2 => ("Yet", Color::SystemBlue),
            _ => unreachable!(),
        };
        let label = Label::new();
        label.set_text(msg.0);
        label.set_background_color(msg.1);
        label.set_text_alignment(cacao::text::TextAlign::Center);
        let row = ListViewRow::new();
        row.add_subview(&label);
        LayoutConstraint::activate(&[
            label.center_x.constraint_equal_to(&row.center_x),
            label.center_y.constraint_equal_to(&row.center_y),
        ]);
        row.set_identifier(msg.0);
        row
    }
}

/// The config window in use.\
/// Parse inut as CLI arguments.
pub struct TextualConfigWindow {
    window: OnceAssign<Window>,

    pub input: TextField,
}

impl TextualConfigWindow {
    pub fn new() -> Self {
        Self {
            window: OnceAssign::new(),
            input: TextField::new(),
        }
    }
}

impl WindowDelegate for TextualConfigWindow {
    const NAME: &'static str = "CORTextualConfigWindowDelegate";

    fn did_load(&mut self, window: Window) {
        self.window.set(window);
        self.window.set_content_size(300, 200);
        self.window.set_title("Preferences");

        self.input.objc.with_mut(|obj| unsafe {
            let focus_ring_type: NSUInteger = FocusRingType::None.into();
            let _: () = msg_send![obj, setFocusRingType:focus_ring_type];
        });

        self.window.set_content_view(&self.input);
    }

    fn will_close(&self) {
        if self.input.get_value() == ACTIVATE {
            self.input.set_text("");
            app_from_objc::<CorApp>().gui_config_window.show();
        }
    }
}
