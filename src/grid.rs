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

#[derive(Debug)]
pub struct ConflictDescriptor<'a> {
    pub locs: &'a [Loc],

    pub available_locs_num: usize,
    /// Number of starting locations.
    ///
    /// Can be 2, 3, or 4.
    pub locs_num: usize,

    /// Ids of the possible opponents.
    pub players: &'a [u32],
    pub ui_players: &'a [u32],

    /// 1, ... number of available locations.
    /// 1 is the best.
    pub conditions: usize,
    /// Inequality from 0 to 4.
    pub ineq: u32,
}

impl Grid {
    /// Creates a new grid with given width and height.
    /// Tiles are generated randomly, including mountains,
    /// mines and cities.
    ///
    /// See [`Tile::new`].
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.min(crate::MAX_WIDTH),
            height: height.min(crate::MAX_HEIGHT),

            tiles: vec![(); width as usize]
                .into_iter()
                .map(|_| {
                    vec![(); height as usize]
                        .into_iter()
                        .map(|_| Tile::new())
                        .collect()
                })
                .collect(),
        }
    }

    pub fn conflict(&mut self, descriptor: ConflictDescriptor<'_>) {
        let ConflictDescriptor {
            locs,
            available_locs_num,
            locs_num,
            players,
            ui_players,
            conditions,
            ineq,
        } = descriptor;

        // Remove all cities.
        for arr in self.tiles.iter_mut() {
            for tile in arr {
                if let Tile::Habitable { land, units, owner } = tile {
                    units.copy_from_slice(&[0; 8]);
                    *owner = Player::NEUTRAL;
                    *land = HabitLand::Grassland;
                }
            }
        }

        let locs_num = in_segment!(locs_num, 2, available_locs_num);
        let num = locs_num.min(players.len() + ui_players.len());
        let di = fastrand::usize(..available_locs_num);

        let mut chosen_locs = vec![Loc(0, 0); num];
        for (i, loc) in chosen_locs.iter_mut().enumerate() {
            let ii = (i + di + available_locs_num) % available_locs_num;
            *loc = locs[ii];
            let Loc(x, y) = *loc;
            self.tiles[x as usize][y as usize].set_habitable(HabitLand::Fortress);

            // Place mines nearby
            let Loc(ri, rj) = fastrand::choice(Loc::DIRS).unwrap();
            self.tiles[(x + ri) as usize][(y + rj) as usize] = Tile::Mine(Player::NEUTRAL);
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

impl From<(u32, u32)> for Loc {
    #[inline]
    fn from((x, y): (u32, u32)) -> Self {
        Self(x as i32, y as i32)
    }
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

    /// Randomly generates a tile from scratch.
    fn new() -> Self {
        let mut this = Self::default();
        match fastrand::u32(..20) {
            0 => {
                this = Tile::Habitable {
                    land: match fastrand::u32(..6) {
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
                this = if fastrand::u32(..10) == 0 {
                    Tile::Mine(Default::default())
                } else {
                    Tile::Mountain
                };
            }
            _ => {
                this.set_owner(Player(fastrand::u32(..crate::MAX_PLAYERS as u32)));
            }
        }

        if this.is_city() {
            if let Tile::Habitable {
                ref mut units,
                owner,
                ..
            } = this
            {
                units[owner.0 as usize] = 10;
            } else {
                unreachable!()
            }
        }

        this
    }

    pub fn set_habitable(&mut self, land: HabitLand) {
        let l = land;
        if let Self::Habitable { land, .. } = self {
            *land = l
        } else {
            *self = Self::Habitable {
                land: l,
                units: [0; MAX_PLAYERS],
                owner: Player::NEUTRAL,
            }
        }
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
#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash, Default)]
pub enum Stencil {
    Rhombus,
    #[default]
    Rect,
    Hex,
}

impl Stencil {
    /// Max count of nations of this stencil.
    pub const fn max_locs(self) -> usize {
        match self {
            Stencil::Rect | Stencil::Rhombus => 4,
            Stencil::Hex => 6,
        }
    }

    /// Applies thie stencil to the given grid and
    /// nation locations slice.
    pub fn apply(self, grid: &mut Grid, d: u32, locs: &mut [Loc]) {
        macro_rules! ij {
            (x, $i:expr, $j:expr) => {
                0.5 * ($j as f32) + ($i as f32)
            };
            (y, $i:expr, $j:expr) => {
                $j as f32
            };
            ($i:expr, $j:expr) => {
                (ij!(x, $i, $j), ij!(y, $i, $j))
            };
        }

        match self {
            Stencil::Rhombus => {
                const LOC_NUM: usize = 4;
                let xs: [_; LOC_NUM] = [d, grid.width - 1 - d, d, grid.width - 1 - d];
                let ys: [_; LOC_NUM] = [d, grid.height - 1 - d, d, grid.height - 1 - d];
                for (loc, xy) in locs.iter_mut().zip(xs.into_iter().zip(ys.into_iter())) {
                    *loc = xy.into();
                }
            }
            Stencil::Rect => {
                const EPSILON: f32 = 0.1;
                let (x0, y0) = (ij!(x, 0, grid.height - 1) - EPSILON, ij!(y, 0, 0) - EPSILON);
                let (x1, y1) = (
                    ij!(x, grid.width - 1, 0) + EPSILON,
                    ij!(y, 0, grid.height - 1) + EPSILON,
                );

                for (i, arr) in grid.tiles.iter_mut().enumerate() {
                    for (j, tile) in arr.iter_mut().enumerate() {
                        let (x, y) = ij!(i, j);
                        if x < x0 || x > x1 || y < y0 || y > y1 {
                            *tile = Tile::Void;
                        }
                    }
                }

                const LOC_NUM: usize = 4;
                let dx = grid.height / 2;
                locs[..LOC_NUM].copy_from_slice(
                    &[
                        (dx + d - 1, d),
                        (grid.width - dx - 1 - d + 1, grid.height - 1 - d),
                        (d + 1, grid.height - 1 - d),
                        (grid.width - 1 - d - 1, d),
                    ]
                    .map(Loc::from),
                )
            }
            Stencil::Hex => {
                let dx = grid.height / 2;
                for (i, arr) in grid
                    .tiles
                    .iter_mut()
                    .enumerate()
                    .map(|(a, b)| (a as u32, b))
                {
                    for (j, tile) in arr.iter_mut().enumerate().map(|(a, b)| (a as u32, b)) {
                        if i + j < dx || i + j > grid.width - 1 + grid.height - 1 - dx {
                            *tile = Tile::Void;
                        }
                    }
                }

                const LOC_NUM: usize = 6;
                locs[..LOC_NUM].copy_from_slice(
                    &[
                        (dx + d - 2, d),
                        (d, grid.height - 1 - d),
                        (grid.width - 1 - d, dx),
                        (d, dx),
                        (grid.width - 1 - d - 2 + 2, d),
                        (grid.width - 1 - dx - d + 2, grid.height - 1 - d),
                    ]
                    .map(Loc::from),
                )
            }
        }
    }
}
