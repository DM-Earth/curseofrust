use cacao::{
    core_graphics::{
        base::CGFloat,
        display::{CGPoint, CGRect, CGSize},
    },
    foundation::{id, NSUInteger},
    image::{Image, ImageView},
    layout::Layout,
    objc::{class, msg_send, runtime::Bool},
};

use super::Texture;

/// Copied from `icrate`.\
/// 2024-07-01 update: `icrate` is dead.
#[allow(non_upper_case_globals)]
const NSCompositingOperationSourceOver: NSUInteger = 2;

thread_local! {
    /// Contains all possible characters of all colors.
    static TYPE: Image = Image::with_data(include_bytes!("../../../images/type.gif"));
    /// The line between two text sections.
    static UI: Image = Image::with_data(include_bytes!("../../../images/ui.gif"));
    /// Main game resources.
    static TILE: Image = Image::with_data(include_bytes!("../../../images/tileset.gif"));
}

pub fn draw_raw(texture: Texture, dest: CGPoint, from: CGRect) {
    unsafe {
        match texture {
                Texture::Type => &TYPE,
                Texture::Ui => &UI,
                Texture::Tile => &TILE,
            }.with(|img| msg_send![&img.0, drawAtPoint:dest fromRect:from, operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)])
    }
}

pub struct Renderer {
    pub view: ImageView,
    pub screen: Option<Image>,
    pub screen_size: CGSize,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            view: ImageView::new(),
            screen: None,
            screen_size: Default::default(),
        }
    }

    #[inline]
    pub fn view(&self) -> &(impl Layout + 'static) {
        &self.view
    }

    pub fn init_renderer(&mut self, screen_size: CGSize) {
        self.screen_size = screen_size;
        unsafe {
            let alloc: id = msg_send![class!(NSImage), alloc];
            let obj: id = msg_send![alloc, initWithSize:screen_size];
            self.screen = Some(Image::with(obj));
        }
        self.view.set_image(self.screen.as_ref().unwrap());
    }

    pub fn finalize_renderer(&mut self) {
        self.screen = None;
    }

    pub fn init_frame(&self) {
        unsafe {
            // Initialize frame rendering
            let background: id = msg_send![class!(NSColor), blackColor];
            let _: () = msg_send![&self.screen.as_ref().unwrap().0, lockFocusFlipped:Bool::YES];
            // Draw background
            let _: () = msg_send![background, drawSwatchInRect:CGRect::new(&CGPoint::new(0., 0.), &self.screen_size)];
        }
    }

    pub fn finalize_frame(&self) {
        unsafe {
            let _: () = msg_send![&self.screen.as_ref().unwrap().0, unlockFocus];
        }
    }
}
