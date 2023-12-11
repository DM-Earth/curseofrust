use crate::*;

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
    pub locs: &'a [Pos],

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

        let mut chosen_locs = vec![Pos(0, 0); num];
        for (i, loc) in chosen_locs.iter_mut().enumerate() {
            let ii = (i + di + available_locs_num) % available_locs_num;
            *loc = locs[ii];
            let Pos(x, y) = *loc;
            self.tiles[x as usize][y as usize].set_habitable(HabitLand::Fortress);

            // Place mines nearby
            let Pos(ri, rj) = fastrand::choice(Pos::DIRS).unwrap();
            self.tiles[(x + ri) as usize][(y + rj) as usize] = Tile::Mine(Player::NEUTRAL);
            self.tiles[(x - 2 * ri) as usize][(y - 2 * rj) as usize] = Tile::Mine(Player::NEUTRAL);
            self.tiles[(x - ri) as usize][(y - rj) as usize] = Tile::Habitable {
                land: HabitLand::Grassland,
                units: [0; 8],
                owner: Player::NEUTRAL,
            };
        }

        let mut eval_result = [0; 7];
        let loc_index = [0, 1, 2, 3, 4, 5, 6];
        self.eval_locs(&chosen_locs, &mut eval_result[..num]);
    }

    fn eval_locs(&self, locs: &[Pos], result: &mut [i32]) {
        let mut u = vec![vec![0; self.height as usize]; self.width as usize];
        let mut d = vec![vec![0; self.height as usize]; self.width as usize];

        const UNREACHABLE: i32 = -1;
        const COMPETITION: i32 = -2;

        for (arr_u, arr_d) in u.iter_mut().zip(d.iter_mut()) {
            for (ui, di) in arr_u.iter_mut().zip(arr_d.iter_mut()) {
                *di = (MAX_WIDTH * MAX_HEIGHT + 1) as i32;
                *ui = UNREACHABLE;
            }
        }

        locs.iter().enumerate().for_each(|(i, &loc)| {
            self.floodfill_closest(&mut u, &mut d, loc, i as i32, 0);
        });

        for (i, arr) in self.tiles.iter().enumerate() {
            for j in arr.iter().enumerate().filter_map(|(i, t)| {
                if matches!(t, Tile::Mine(_)) {
                    Some(i)
                } else {
                    None
                }
            }) {
                let mut max_dist = 0;
                let mut min_dist = (MAX_WIDTH * MAX_HEIGHT + 1) as i32;

                let mut single_owner = UNREACHABLE;

                for (x, y) in Pos::DIRS.into_iter().filter_map(|Pos(x, y)| {
                    let (x, y) = (i as i32 + x, j as i32 + y);
                    if x < 0
                        || x >= self.width as i32
                        || y < 0
                        || y >= self.height as i32
                        || !self.tiles[x as usize][y as usize].is_habitable()
                    {
                        None
                    } else {
                        Some((x as usize, y as usize))
                    }
                }) {
                    let dd = d[x][y];
                    let uu = u[x][y];
                    if single_owner == UNREACHABLE {
                        single_owner = uu;
                        max_dist = dd;
                        min_dist = dd;
                    } else if uu == single_owner {
                        max_dist = max_dist.max(dd);
                        min_dist = min_dist.min(dd);
                    } else if uu != UNREACHABLE {
                        single_owner = COMPETITION
                    }
                }

                if single_owner != COMPETITION && single_owner != UNREACHABLE {
                    result[single_owner as usize] += (100.0
                        * (MAX_WIDTH + MAX_HEIGHT) as f32
                        * (-10.0 * (max_dist * min_dist) as f32 / (MAX_WIDTH * MAX_HEIGHT) as f32)
                            .exp()) as i32;
                }
            }
        }
    }

    /// Floodfill with value `val`, the closest
    /// distance has priority.
    fn floodfill_closest(
        &self,
        u: &mut [Vec<i32>],
        d: &mut [Vec<i32>],
        Pos(x, y): Pos,
        val: i32,
        dist: i32,
    ) {
        if x < 0
            || x >= self.width as i32
            || y < 0
            || y >= self.height as i32
            || self.tiles[x as usize][y as usize].is_habitable()
        {
            return;
        }

        u[x as usize][y as usize] = val;
        d[x as usize][y as usize] = dist;

        for &Pos(dx, dy) in Pos::DIRS.iter() {
            self.floodfill_closest(u, d, Pos(x + dx, y + dy), val, dist + 1);
        }
    }
}

fn sort(vals: &[i32], items: &[i32])

/// A location.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Pos(
    /// Horizontal axis.
    i32,
    /// Vertical axis.
    i32,
);

impl Pos {
    /// Possible directions to move a [`Tile`].
    pub const DIRS: [Self; 6] = [
        Self(-1, 0),
        Self(1, 0),
        Self(0, -1),
        Self(0, 1),
        Self(1, -1),
        Self(-1, 1),
    ];
}

impl From<(u32, u32)> for Pos {
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
    pub fn apply(self, grid: &mut Grid, d: u32, locs: &mut [Pos]) {
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
                    .map(Pos::from),
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
                    .map(Pos::from),
                )
            }
        }
    }
}
