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

pub mod grid;
pub mod king;
pub mod state;

pub const MAX_WIDTH: u32 = 40;
pub const MAX_HEIGHT: u32 = 29;

pub const MAX_PLAYERS: usize = 8;
pub const MAX_POPULATION: u16 = 499;

pub use grid::{FlagGrid, Grid, Pos, FLAG_POWER};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Player(u32);

impl Player {
    pub const NEUTRAL: Self = Self(0);

    #[inline]
    pub fn is_neutral(self) -> bool {
        self == Self::NEUTRAL
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_neutral() {
            write!(f, "neutral")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Difference of evaluation result and population variance
    /// out of bound in the `conflict` function.
    ///
    /// See [`Grid::conflict`].
    ConflictDiffOutOfBound,
    /// Position out of height or width bounds.
    PosOutOfBound(Pos),

    /// Operating player is not owner of the tile.
    NotOwner {
        operator: Player,
        owner: Player,
        tile: Pos,
    },
    /// The target tile is not habitable.
    TileNotHabitable(Pos),
    /// Trying to upgrade a fortress, which
    /// cannot be upgraded anymore.
    UpgradeTopLevelBuilding,
    /// Trying to degrade grassland, which
    /// cannot be degraded anymore.
    DegradeGrassLand,
    /// Money not enough.
    InsufficientGold {
        required: u64,
        /// Gold player already has.
        owning: u64,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConflictDiffOutOfBound => write!(
                f,
                "difference of evaluation result and population variance out of bound"
            ),
            Error::PosOutOfBound(pos) => {
                write!(f, "location {pos:?} out of width and height bounds")
            }
            Error::NotOwner {
                operator,
                owner,
                tile,
            } => write!(
                f,
                "{operator} is not the owner of tile {tile:?} (owner: {owner})"
            ),
            Error::TileNotHabitable(pos) => write!(f, "tile {pos:?} is not habitable"),
            Error::UpgradeTopLevelBuilding => write!(f, "upgrading a fortress is illegal"),
            Error::DegradeGrassLand => write!(f, "degrading grassland is illegal"),
            Error::InsufficientGold { required, owning } => write!(
                f,
                "gold not enough: required {required}, player owns {owning}"
            ),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Game speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Speed {
    Pause,
    Slowest,
    Slower,
    Slow,
    Normal,
    Faster,
    Fastest,
}

/// Game difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    Easiest,
    Easy,
    Normal,
    Hard,
    Hardest,
}
