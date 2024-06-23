//! Curseofwar messaging protocol implementation.

use bytemuck::{AnyBitPattern, NoUninit, Zeroable};
use curseofrust::{
    grid::{HabitLand, Tile},
    MAX_HEIGHT, MAX_PLAYERS, MAX_WIDTH,
};

use std::mem::offset_of;

mod client;
mod server;

pub use client::*;
pub use server::*;

/// Data structure a client transferred to a server.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct C2SData {
    /// The targeting X position.
    pub x: u8,
    /// The targeting Y position.
    pub y: u8,
    /// The message.
    #[doc(alias = "info")]
    pub msg: u8,
}

pub const C2S_SIZE: usize = std::mem::size_of::<C2SData>() + 1;

#[repr(C)]
#[allow(dead_code)]
struct UnsafeC2SData {
    x: u8,
    y: u8,
    #[doc(alias = "info")]
    msg: u8,
}

/// Message a client transferred to a server.
pub mod client_msg {
    pub const CONNECT: u8 = 1;
    pub const BUILD: u8 = 20;

    pub const FLAG_ON: u8 = 21;
    pub const FLAG_OFF: u8 = 22;
    pub const FLAG_OFF_ALL: u8 = 23;
    pub const FLAG_OFF_HALF: u8 = 24;

    pub const IS_ALIVE: u8 = 30;
    pub const PAUSE: u8 = 40;
    pub const UNPAUSE: u8 = 41;
}

/// Message a server transferred to a client.
pub mod server_msg {
    pub const CONN_ACCEPTED: u8 = 5;
    pub const CONN_REJECTED: u8 = 6;

    pub const STATE: u8 = 10;
}

/// Class of tiles.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum TileClass {
    #[doc(alias = "Abyss")]
    Void = 0,
    Mountain = 1,
    Mine = 2,
    Grassland = 3,
    Village = 4,
    Town = 5,
    #[doc(alias = "Castle")]
    Fortress = 6,
}

impl From<&Tile> for TileClass {
    #[inline]
    fn from(value: &Tile) -> Self {
        match value {
            Tile::Void => TileClass::Void,
            Tile::Mountain => TileClass::Mountain,
            Tile::Mine(_) => TileClass::Mine,
            Tile::Habitable { land, .. } => match land {
                HabitLand::Fortress => TileClass::Fortress,
                HabitLand::Town => TileClass::Town,
                HabitLand::Village => TileClass::Village,
                HabitLand::Grassland => TileClass::Grassland,
            },
        }
    }
}

impl TryFrom<u8> for TileClass {
    type Error = ();

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => TileClass::Void,
            1 => TileClass::Mountain,
            2 => TileClass::Mine,
            3 => TileClass::Grassland,
            4 => TileClass::Village,
            5 => TileClass::Town,
            6 => TileClass::Fortress,
            _ => return Err(()),
        })
    }
}

impl From<TileClass> for Tile {
    #[inline]
    fn from(value: TileClass) -> Self {
        match value {
            TileClass::Void => Tile::Void,
            TileClass::Mountain => Tile::Mountain,
            TileClass::Mine => Tile::Mine(Default::default()),
            TileClass::Grassland | TileClass::Village | TileClass::Town | TileClass::Fortress => {
                Tile::Habitable {
                    land: match value {
                        TileClass::Grassland => HabitLand::Grassland,
                        TileClass::Village => HabitLand::Village,
                        TileClass::Town => HabitLand::Town,
                        TileClass::Fortress => HabitLand::Fortress,
                        _ => unreachable!(),
                    },
                    units: [0u16; MAX_PLAYERS],
                    owner: Default::default(),
                }
            }
        }
    }
}

/// Data structure a server transferred to a client.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct S2CData {
    /// Player you control.
    #[doc(alias = "control")]
    pub player: u8,
    /// Pause request.
    pub pause_request: u8,
    __pad0: [u8; __S2C_PAD_0_LEN],

    /// Gold counts.
    pub gold: [u32; MAX_PLAYERS],
    /// Current time.
    pub time: u32,

    /// Width of the grid.
    pub width: u8,
    /// Height of the grid.
    pub height: u8,
    /// Flag powers of the grid.
    pub flag: [[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    /// Owner of each grid.
    pub owner: [[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    __pad1: [u8; __S2C_PAD_1_LEN],
    /// Population of each grid.
    pub pop: [[u16; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    /// Population of each grid.
    pub tile: [[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    __pad2: [u8; __S2C_PAD_2_LEN],
}

#[repr(C)]
struct UnsafeS2CData {
    player: u8,
    pause_request: u8,
    gold: [u32; MAX_PLAYERS],
    time: u32,
    width: u8,
    height: u8,
    flag: [[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    owner: [[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    pop: [[u16; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
    tile: [[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize],
}

const __S2C_PAD_0_LEN: usize = offset_of!(UnsafeS2CData, gold)
    - offset_of!(UnsafeS2CData, pause_request)
    - std::mem::size_of::<u8>();
const __S2C_PAD_1_LEN: usize = offset_of!(UnsafeS2CData, pop)
    - offset_of!(UnsafeS2CData, owner)
    - std::mem::size_of::<[[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize]>();
const __S2C_PAD_2_LEN: usize = std::mem::size_of::<UnsafeS2CData>()
    - offset_of!(UnsafeS2CData, tile)
    - std::mem::size_of::<[[u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize]>();

//SAFETY: `C2SData` and `S2CData` are manually padded.
unsafe impl Zeroable for C2SData {}
unsafe impl AnyBitPattern for C2SData {}
unsafe impl NoUninit for C2SData {}
unsafe impl Zeroable for S2CData {}
unsafe impl AnyBitPattern for S2CData {}
unsafe impl NoUninit for S2CData {}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn s2c_data_layout() {
        assert_eq!(
            std::mem::size_of::<S2CData>(),
            std::mem::size_of::<UnsafeS2CData>()
        );

        macro_rules! assert_offset_eq {
            ($($f:ident),*$(,)?) => {
                $(
                assert_eq!(
                    std::mem::offset_of!(S2CData, $f),
                    std::mem::offset_of!(UnsafeS2CData, $f),
                );
                )*
            };
        }

        assert_offset_eq! {
            player,
            pause_request,
            gold,
            time,
            width,
            height,
            flag,
            owner,
            pop,
            tile,
        }
    }

    #[test]
    fn c2s_data_layout() {
        assert_eq!(
            std::mem::size_of::<C2SData>(),
            std::mem::size_of::<UnsafeC2SData>()
        );

        macro_rules! assert_offset_eq {
            ($($f:ident),*$(,)?) => {
                $(
                assert_eq!(
                    std::mem::offset_of!(C2SData, $f),
                    std::mem::offset_of!(UnsafeC2SData, $f),
                );
                )*
            };
        }

        assert_offset_eq! {
            x,
            y,
            msg,
        }
    }
}
