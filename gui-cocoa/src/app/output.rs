use cacao::{
    core_graphics::{
        base::CGFloat,
        geometry::{CGPoint, CGRect, CGSize},
    },
    foundation::NSUInteger,
    image::Image,
    lazy_static::lazy_static,
    objc::{msg_send, sel, sel_impl},
};
use curseofrust::{state, Grid, Player, Pos};

lazy_static! {
    /// Contains all possible characters of all colors.
    static ref TYPE:Image = Image::with_data(include_bytes!("../../images/type.gif"));
    /// The line between two text sections.
    static ref UI:Image=Image::with_data(include_bytes!("../../images/ui.gif"));
    /// Main game resources.
    static ref TILE:Image=Image::with_data(include_bytes!("../../images/tileset.gif"));
}

pub const TILE_WIDTH: i16 = 32;
pub const TILE_HEIGHT: i16 = 16;
pub const TYPE_WIDTH: i16 = 9;
pub const TYPE_HEIGHT: i16 = 15;

const TYPE_FIRST: u8 = 33;
const LINE_LENGTH: i16 = 32;
const COLOR_OFFSET: i16 = 3;

/// Copied from `icrate`.
#[allow(non_upper_case_globals)]
const NSCompositingOperationSourceOver: NSUInteger = 2;

/// Magic, but it works fine.
#[inline]
pub fn pos_x(ui: &state::UI, i: i16) -> i16 {
    i - ui.xskip as i16
}

#[inline]
pub fn pos_y(j: i16) -> i16 {
    j + 1
}

/// Draws string with specified color.\
/// All chars in `string` should be ascii char.\
/// You should call `lockFocusFlipped:YES` before calling this.
pub fn draw_str(string: &str, color: Player, dest_x: i16, dest_y: i16) {
    let offset: i16 = match color {
        Player::NEUTRAL => 0,
        Player(x) => COLOR_OFFSET + x as i16,
    };
    for (index, c) in string.char_indices() {
        if c == ' ' {
            continue;
        }
        let ascii = (c as u8 - TYPE_FIRST) as i16;
        let i = ascii % LINE_LENGTH;
        let j = ascii / LINE_LENGTH + offset;
        let type_rect = CGRect::new(
            &CGPoint::new((i * TYPE_WIDTH) as f64, (j * TYPE_HEIGHT) as f64),
            &CGSize::new(TYPE_WIDTH as f64, TYPE_HEIGHT as f64),
        );
        let dest_point = CGPoint::new((dest_x + index as i16 * TYPE_WIDTH) as f64, dest_y as f64);
        let _: () = unsafe {
            msg_send![TYPE.0, drawAtPoint:dest_point fromRect:type_rect operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)]
        };
    }
}

/// Draws common tiles like grassland.
pub fn draw_tile(src_i: i16, src_j: i16, dest_i: i16, dest_j: i16) {
    let tile_rect = CGRect::new(
        &CGPoint::new((src_i * TILE_WIDTH) as f64, (src_j * TILE_HEIGHT) as f64),
        &CGSize::new(TILE_WIDTH as f64, TILE_HEIGHT as f64),
    );
    let dest_point = CGPoint::new(
        (dest_i * TILE_WIDTH + dest_j * TILE_WIDTH / 2) as f64,
        (dest_j * TILE_HEIGHT) as f64,
    );
    let _: () = unsafe {
        msg_send![TILE.0, drawAtPoint:dest_point fromRect:tile_rect operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)]
    };
}

/// Draws double height tiles like working mine.
pub fn draw_tile_2h(src_i: i16, src_j: i16, dest_i: i16, dest_j: i16) {
    let tile_rect = CGRect::new(
        &CGPoint::new(
            (src_i * TILE_WIDTH) as f64,
            ((src_j - 1) * TILE_HEIGHT) as f64,
        ),
        &CGSize::new(TILE_WIDTH as f64, (TILE_HEIGHT * 2) as f64),
    );
    let dest_point = CGPoint::new(
        (dest_i * TILE_WIDTH + dest_j * TILE_WIDTH / 2) as f64,
        ((dest_j - 1) * TILE_HEIGHT) as f64,
    );
    let _: () = unsafe {
        msg_send![TILE.0, drawAtPoint:dest_point fromRect:tile_rect operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)]
    };
}

/// Draws tiles with offset like population.
pub fn draw_tile_noise(src_i: i16, src_j: i16, dest_i: i16, dest_j: i16, var: i16) {
    let tile_rect = CGRect::new(
        &CGPoint::new((src_i * TILE_WIDTH) as f64, (src_j * TILE_HEIGHT) as f64),
        &CGSize::new(TILE_WIDTH as f64, TILE_HEIGHT as f64),
    );
    let rnd_x = var % 3 - 1;
    let rnd_y = var % 2;
    let dest_point = CGPoint::new(
        ((dest_i * TILE_WIDTH + dest_j * TILE_WIDTH / 2) + rnd_x) as f64,
        ((dest_j * TILE_HEIGHT) + rnd_y) as f64,
    );
    let _: () = unsafe {
        msg_send![TILE.0, drawAtPoint:dest_point fromRect:tile_rect operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)]
    };
}

/// Return value:
/// 1. left top
/// 2. right top
/// 3. left bottom
/// 4. right bottom
pub fn is_cliff(i: i16, j: i16, grid: &Grid) -> [bool; 4] {
    let mut ret = [false, false, false, false];
    if !is_normal(i, j, grid) {
        let left = is_normal(i - 1, j, grid);
        let right = is_normal(i + 1, j, grid);
        let top = is_normal(i, j - 1, grid) || is_normal(i + 1, j - 1, grid);
        let bottom = is_normal(i, j + 1, grid) || is_normal(i - 1, j + 1, grid);
        ret[0] = left && top;
        ret[1] = right && top;
        ret[2] = left && bottom;
        ret[3] = right && bottom;
    }
    ret
}

#[inline]
pub fn is_normal(i: i16, j: i16, grid: &Grid) -> bool {
    grid.tile(Pos(i as i32, j as i32))
        .is_some_and(|t| t.is_visible())
}

#[inline]
pub fn is_within_grid(i: i16, j: i16, grid: &Grid) -> bool {
    grid.tile(Pos(i as i32, j as i32)).is_some()
}

pub fn pop_to_symbol(pop: u16) -> i16 {
    match pop {
        0 => -1,
        1..=3 => 0,
        4..=6 => 1,
        7..=12 => 2,
        13..=25 => 3,
        26..=50 => 4,
        51..=100 => 5,
        101..=200 => 6,
        201..=400 => 7,
        401.. => 8,
    }
}

/// Just port the C version.\
/// Pretty rough though.
pub fn time_to_ymd(time: u64) -> (u64, u64, u64) {
    let year = time / 360;
    let mut month = time - year * 360;
    let day = month % 30 + 1;
    month = month / 30 + 1;
    (year, month, day)
}

pub fn draw_line(base_y: i16) {
    const LINE_WIDTH: f64 = 555.;
    let ui_rect = CGRect::new(&CGPoint::new(0., 0.), &CGSize::new(LINE_WIDTH, 1.));
    let dest_point = CGPoint::new(
        TILE_WIDTH as f64 + 75. * TYPE_WIDTH as f64 / 2. - LINE_WIDTH / 2.,
        base_y as f64 + TYPE_HEIGHT as f64 * 5. / 2.,
    );
    let _: () = unsafe {
        msg_send![UI.0, drawAtPoint:dest_point fromRect:ui_rect operation:NSCompositingOperationSourceOver fraction:(1. as CGFloat)]
    };
}
