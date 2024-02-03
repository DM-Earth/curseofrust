use cacao::{
    appkit::window::{Window, WindowDelegate},
    button::Button,
    listview::{ListView, ListViewDelegate, ListViewRow},
};

use crate::util::OnceAssign;

/// Uses `preferences` as a replacement of `config` to fit the
/// Mac OS X language style.
pub struct ConfigWindow {
    window: OnceAssign<Window>,

    list: ListView,
}

impl ConfigWindow {
    pub fn new() -> Self {
        Self {
            window: OnceAssign::new(),
            list: ListView::new(),
        }
    }
}

impl WindowDelegate for ConfigWindow {
    const NAME: &'static str = "CORConfigWindowDelegate";

    fn did_load(&mut self, window: Window) {
        self.window.set(window);
        self.window.set_content_size(300, 200);
        self.window.set_title("Preferences");
        self.window.set_content_view(&self.list);
    }
}

/// List containing config items in the config window.
struct ConfigList {
    list: OnceAssign<ListView>,
}

impl ConfigList {
    fn new() -> Self {
        todo!();
    }
}

impl ListViewDelegate for ConfigList {
    const NAME: &'static str = "CORConfigListViewDelegate";

    fn did_load(&mut self, view: ListView) {
        self.list.set(view);
        todo!()
    }

    fn number_of_items(&self) -> usize {
        17
    }

    fn item_for(&self, row: usize) -> ListViewRow {
        todo!()
    }
}
