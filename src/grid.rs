use crate::{Player, MAX_PLAYERS};

/// 2D array of tiles with width and height
/// information.
///
/// The map is stored in this type.
#[derive(Debug)]
pub struct Grid {
    width: u32,
    height: u32,

    /// 2 dimensional tiles, as `[x][y]`.
    tiles: Vec<Vec<Tile>>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.min(crate::MAX_WIDTH),
            height: height.min(crate::MAX_HEIGHT),

            tiles: vec![(); width as usize]
                .into_iter()
                .map(|_| {
                    vec![(); height as usize]
                        .into_iter()
                        .map(|_| Tile::rand())
                        .collect()
                })
                .collect(),
        }
    }
}

/// A location.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Loc(
    /// Horizontal axis.
    i32,
    /// Vertical axis.
    i32,
);

impl Loc {
    /// Possible derections to move a [`Tile`].
    pub const DIRS: [Self; 6] = [
        Self(-1, 0),
        Self(1, 0),
        Self(0, -1),
        Self(0, 1),
        Self(1, -1),
        Self(-1, 1),
    ];
}

#[derive(Debug, Clone)]
pub enum Tile {
    /// Abyss.
    Void,
    /// Natural barrier.
    Mountain,
    /// Source of gold.
    Mine(Player),
    /// Habitable territory.
    Habitable {
        land: HabitLand,
        /// Population information of this tile.
        units: [u16; MAX_PLAYERS],
        owner: Player,
    },
}

impl Tile {
    #[inline]
    pub fn owner(&self) -> Player {
        match self {
            Self::Mine(p) => *p,
            Self::Habitable { owner, .. } => *owner,
            _ => Default::default(),
        }
    }

    #[inline]
    pub fn set_owner(&mut self, player: Player) {
        match self {
            Self::Mine(p) => *p = player,
            Self::Habitable { owner, .. } => *owner = player,
            _ => (),
        }
    }

    /// Randomly generate a tile from scratch.
    fn rand() -> Self {
        let mut this = Self::default();
        match fastrand::u32(..) % 20 {
            0 => {
                this = Tile::Habitable {
                    land: match fastrand::u32(..) % 6 {
                        0 => HabitLand::Fortress,
                        1 | 2 => HabitLand::Town,
                        _ => HabitLand::Village,
                    },
                    units: [0; MAX_PLAYERS],
                    owner: Default::default(),
                }
            }
            ..=4 => {
                // Mountains and mineis
                this = if fastrand::u32(..) % 10 == 0 {
                    Tile::Mine(Default::default())
                } else {
                    Tile::Mountain
                };
            }
            _ => {
                let x = 1 + fastrand::u32(..) % (crate::MAX_PLAYERS as u32 - 1);
                if x < crate::MAX_PLAYERS as u32 {
                    this.set_owner(x.into())
                } else {
                    this.set_owner(Default::default())
                }
            }
        }

        if this.is_city() {
            if let Tile::Habitable {
                ref mut units,
                owner,
                ..
            } = this
            {
                units[u32::from(owner) as usize] = 10;
            } else {
                unreachable!()
            }
        }

        this
    }
}

impl Default for Tile {
    #[inline]
    fn default() -> Self {
        Self::Habitable {
            land: Default::default(),
            units: [0; MAX_PLAYERS],
            owner: Default::default(),
        }
    }
}

/// Habitable tile variants.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum HabitLand {
    /// Habitable territory that does not have cities.
    #[default]
    Grassland,
    Village,
    Town,
    /// Castles.
    Fortress,
}

impl Tile {
    /// Whether this tile is inhabitable.
    #[inline]
    pub fn is_habitable(&self) -> bool {
        matches!(self, Self::Habitable { .. })
    }

    /// Whether this tile is a city.
    pub fn is_city(&self) -> bool {
        if let Self::Habitable { land, .. } = self {
            !matches!(land, HabitLand::Grassland)
        } else {
            false
        }
    }

    /// Whether this tile is visible.
    #[inline]
    pub fn is_visible(&self) -> bool {
        matches!(self, Self::Void)
    }
}

/// Shape of the map.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum Stencil {
    Rect,
    Rhombus,
    Hex,
}

impl Stencil {
    /// Max count of nations of this stencil.
    #[inline]
    pub fn max_nations(self) -> usize {
        match self {
            Stencil::Rect | Stencil::Rhombus => 4,
            Stencil::Hex => 6,
        }
    }
}
