use cacao::{
    core_graphics::{
        base::CGFloat,
        display::{CGPoint, CGRect},
    },
    foundation::NSUInteger,
    image::Image,
    objc::msg_send,
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
                Texture::Ui=>&UI,
                Texture::Tile=>&TILE,
            }.with(|img| msg_send![&img.0, drawAtPoint:dest fromRect:from, operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)])
    }
}
