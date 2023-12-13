use std::fmt::Display;

macro_rules! in_segment {
    ($x:expr, $l:expr, $r:expr) => {
        if $x < $l {
            $l
        } else if $x > $r {
            $r
        } else {
            $x
        }
    };
}

mod grid;
mod state;

const MAX_WIDTH: u32 = 40;
const MAX_HEIGHT: u32 = 29;

const MAX_PLAYERS: usize = 8;

pub use grid::Grid;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Player(u32);

impl Player {
    pub const NEUTRAL: Self = Self(0);

    #[inline]
    pub fn is_neutral(self) -> bool {
        self == Self::NEUTRAL
    }
}

#[derive(Debug)]
pub enum Error {
    /// Difference of evaluation result and population variance
    /// out of bound in the `conflict` function.
    ///
    /// See [`Grid::conflict`].
    ConflictDiffOutOfBound,
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConflictDiffOutOfBound => {
                f.write_str("difference of evaluation result and population variance out of bound.")
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

fn main() {
    println!("Hello, world!");
}
