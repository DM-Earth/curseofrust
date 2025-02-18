use std::ops::IndexMut;

use crate::*;

pub const FLAG_POWER: i32 = 8;

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

/// Descriptor for method [`Grid::conflict`].
#[derive(Debug)]
pub struct ConflictDescriptor<'a> {
    pub locs: &'a [Pos],

    /// Number of starting locations.
    ///
    /// Can be 2, 3, or 4.
    pub locs_num: usize,

    /// Ids of the possible opponents.
    pub players: &'a [Player],
    pub ui_players: &'a [Player],

    /// 1, ... number of available locations.
    ///
    /// 1 is the best.
    pub conditions: Option<u32>,
    /// Inequality from 0 to 4.
    ///
    /// `None` leaves for a randomly generated value.
    pub ineq: Option<u32>,
}

impl Grid {
    /// Creates a new grid with given width and height.
    /// Tiles are generated randomly, including mountains,
    /// mines and cities.
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let width = width.min(MAX_WIDTH);
        let height = height.min(MAX_HEIGHT);

        Self {
            width,
            height,

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

    /// Gets width of this grid.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Gets height of this grid.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Gets the tile from given position.
    #[inline]
    pub fn tile(&self, Pos(x, y): Pos) -> Option<&Tile> {
        self.tiles.get(x as usize).and_then(|a| a.get(y as usize))
    }

    /// Gets the tile from given position, mutably.
    #[inline]
    pub fn tile_mut(&mut self, Pos(x, y): Pos) -> Option<&mut Tile> {
        self.tiles
            .get_mut(x as usize)
            .and_then(|a| a.get_mut(y as usize))
    }

    /// Gets the raw tiles array of this grid.
    #[inline]
    pub fn raw_tiles(&self) -> &[Vec<Tile>] {
        &self.tiles
    }

    /// Gets the raw tiles array of this grid, mutably.
    #[inline]
    pub fn raw_tiles_mut(&mut self) -> &mut [Vec<Tile>] {
        &mut self.tiles
    }

    /// Enhances an already initialized grid.
    ///
    /// Places at most 4 players at the corners of the map,
    /// gives them a fortress and 2 mines nearby.
    /// One of those players is always controlled by a human player.
    pub fn conflict(&mut self, descriptor: ConflictDescriptor<'_>) -> crate::Result<()> {
        let ConflictDescriptor {
            locs,
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

        let locs_num = in_segment!(locs_num, 2, locs.len());
        let num = locs_num.min(players.len() + ui_players.len());
        let di = fastrand::usize(..locs.len());

        let mut chosen_locs = vec![Pos(0, 0); num];
        for (i, loc) in chosen_locs.iter_mut().enumerate() {
            *loc = locs[(i + di) % locs.len()];
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
        self.eval_locs(&chosen_locs, &mut eval_result[..num]);
        let mut loc_index: [usize; 7] = [0, 1, 2, 3, 4, 5, 6];
        loc_index[..num].sort_by_key(|i| eval_result[*i]);
        eval_result[..num].sort();

        if let Some(ineq) = ineq {
            let avg = eval_result.into_iter().sum::<i32>() as f32 / num as f32;
            // Population variance.
            let var = eval_result
                .into_iter()
                .map(|val| (val as f32 - avg).powi(2))
                .sum::<f32>()
                / num as f32;

            let x = var.sqrt() * 1000.0 / avg;
            if !matches!(
                (ineq, x as i32),
                (0, ..=50) | (1, 51..=100) | (2, 101..=250) | (3, 251..=500) | (4, 501..)
            ) {
                return Err(Error::ConflictDiffOutOfBound);
            }
        }

        // Suffled computer players.
        let mut sh_players_comp = players.to_vec();
        fastrand::shuffle(&mut sh_players_comp);
        let sh_players_comp = sh_players_comp;

        // Shuffled copy of the players array.
        let mut sh_players = ui_players.to_vec();
        let (p0, p1) = sh_players_comp.split_at(fastrand::usize(..players.len()));
        sh_players.extend_from_slice(p1);
        sh_players.extend_from_slice(p0);
        fastrand::shuffle(&mut sh_players[..num]);

        // Human player index.
        let ihuman = conditions.map_or_else(
            || fastrand::u32(..num as u32),
            // Choose specific conditions {1,... N}, 1 => best, N => worst
            |c| loc_index[(num - c as usize).min(num - 1)] as u32,
        );

        for (i, ii) in loc_index[..num].iter().copied().enumerate() {
            let Pos(x, y) = chosen_locs[ii];
            let tile = &mut self.tiles[x as usize][y as usize];
            if ui_players.len() > 1 {
                tile.set_owner(sh_players[i]);
            } else if ii as u32 == ihuman {
                tile.set_owner(ui_players[0]);
            } else {
                tile.set_owner(sh_players_comp[i]);
            }

            let Player(owner) = tile.owner();
            if let Tile::Habitable { units, .. } = tile {
                units[owner as usize] = 10;
            }
        }

        Ok(())
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
            || !self.tiles[x as usize][y as usize].is_habitable()
            || d[x as usize][y as usize] <= dist
        {
            return;
        }

        u[x as usize][y as usize] = val;
        d[x as usize][y as usize] = dist;

        for Pos(dx, dy) in Pos::DIRS {
            self.floodfill_closest(u, d, Pos(x + dx, y + dy), val, dist + 1);
        }
    }

    fn floodfill(&self, u: &mut [Vec<i32>], Pos(x, y): Pos, val: i32) {
        if x < 0
            || x >= self.width as i32
            || y < 0
            || y >= self.height as i32
            || !self.tiles[x as usize][y as usize].is_habitable()
            || u[x as usize][y as usize] == val
        {
            return;
        }
        u[x as usize][y as usize] = val;
        for Pos(dx, dy) in Pos::DIRS {
            self.floodfill(u, Pos(x + dx, y + dy), val)
        }
    }

    /// Returns connectedness of this grid.
    pub fn is_connected(&self) -> bool {
        let mut colored = false;
        let mut m = vec![vec![0; self.height as usize]; self.width as usize];
        for (i, arr_g) in self.tiles.iter().enumerate() {
            for j in arr_g.iter().enumerate().filter_map(|(j, t)| {
                if t.owner().is_neutral() {
                    None
                } else {
                    Some(j)
                }
            }) {
                if colored && m[i][j] == 0 {
                    return false;
                }
                colored = true;
                self.floodfill(&mut m, Pos(i as i32, j as i32), 1)
            }
        }
        true
    }
}

/// A location.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Default)]
pub struct Pos(
    /// Horizontal axis.
    pub i32,
    /// Vertical axis.
    pub i32,
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
#[non_exhaustive]
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
    pub(crate) fn new() -> Self {
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
            1..=4 => {
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

    #[inline]
    pub fn units(&self) -> &[u16; MAX_PLAYERS] {
        if let Self::Habitable { units, .. } = self {
            units
        } else {
            const EMPTY: [u16; MAX_PLAYERS] = [0; MAX_PLAYERS];
            &EMPTY
        }
    }

    #[inline]
    pub fn units_mut(&mut self) -> Option<&mut [u16; MAX_PLAYERS]> {
        if let Self::Habitable { units, .. } = self {
            Some(units)
        } else {
            None
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
#[repr(u8)]
#[non_exhaustive]
pub enum HabitLand {
    /// Habitable territory that does not have cities.
    #[default]
    Grassland,
    Village,
    Town,
    /// Castles.
    Fortress,
}

impl HabitLand {
    /// Gets price of this type of land.
    #[inline]
    pub const fn price(self) -> u64 {
        match self {
            HabitLand::Grassland => 0,
            HabitLand::Village => king::PRICE_VILLAGE,
            HabitLand::Town => king::PRICE_TOWN,
            HabitLand::Fortress => king::PRICE_FORTRESS,
        }
    }

    /// Upgrade this land and returns the upgrade price.
    #[inline]
    pub fn upgrade(&mut self) -> Option<u64> {
        *self = match self {
            HabitLand::Grassland => HabitLand::Village,
            HabitLand::Village => HabitLand::Town,
            HabitLand::Town => HabitLand::Fortress,
            HabitLand::Fortress => return None,
        };
        Some(self.price())
    }

    /// Degrades this land and returns whether
    /// the degrade was successful.
    #[inline]
    pub fn degrade(&mut self) -> bool {
        *self = match self {
            HabitLand::Grassland => return false,
            HabitLand::Village => HabitLand::Grassland,
            HabitLand::Town => HabitLand::Village,
            HabitLand::Fortress => HabitLand::Town,
        };
        true
    }

    pub const fn growth(self) -> f32 {
        match self {
            HabitLand::Village => 1.10,
            HabitLand::Town => 1.20,
            HabitLand::Fortress => 1.30,
            _ => 0.0,
        }
    }
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
        !matches!(self, Self::Void)
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

pub const MAX_AVLBL_LOCS: usize = 7;

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

/// [`Grid`] but stores information about
/// player's flags.
///
/// Each player has his own flag grid.
#[derive(Debug, Clone)]
pub struct FlagGrid {
    pub width: u32,
    pub height: u32,

    /// Whether a position has a flag.
    pub flags: Vec<Vec<bool>>,

    /// Information of power of attraction
    /// a position has.
    ///
    /// Must be updated when flags are added
    /// or removed.
    pub call: Vec<Vec<i32>>,
}

impl FlagGrid {
    /// Creates an empty flag grid with
    /// given width and height.
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.min(MAX_WIDTH);
        let height = height.min(MAX_HEIGHT);

        Self {
            width,
            height,
            flags: vec![vec![false; height as usize]; width as usize],
            call: vec![vec![0; height as usize]; width as usize],
        }
    }

    /// Adds a flag on the given position with the given power.
    pub fn add(&mut self, grid: &Grid, Pos(x, y): Pos, power: i32) {
        let (xu, yu) = (x as usize, y as usize);

        if x < 0
            || x >= self.width as i32
            || y < 0
            || y >= self.height as i32
            || !grid.tiles[xu][yu].is_habitable()
            || self.flags[xu][yu]
        {
            return;
        }

        let mut u = [[0; MAX_HEIGHT as usize]; MAX_WIDTH as usize];
        self.flags[xu][yu] = true;
        grid.spread(&mut u, &mut self.call, Pos(x, y), power, 1);
    }

    /// Removes a flag on the given position with the given power.
    pub fn remove(&mut self, grid: &Grid, Pos(x, y): Pos, power: i32) {
        let (xu, yu) = (x as usize, y as usize);

        if x < 0
            || x >= self.width as i32
            || y < 0
            || y >= self.height as i32
            || !grid.tiles[xu][yu].is_habitable()
            || !self.flags[xu][yu]
        {
            return;
        }

        let mut u = [[0; MAX_HEIGHT as usize]; MAX_WIDTH as usize];
        self.flags[xu][yu] = false;
        grid.spread(&mut u, &mut self.call, Pos(x, y), power, -1);
    }

    /// Iterates over all tiles and removes flags
    /// with probability `prob`.
    ///
    /// With `prob = 1`, all flags will be removed.
    pub fn remove_with_prob(&mut self, grid: &Grid, prob: f32) {
        for i in 0..self.width as i32 {
            for j in 0..self.height as i32 {
                if self.flags[i as usize][j as usize] && fastrand::f32() <= prob {
                    self.remove(grid, Pos(i, j), FLAG_POWER);
                }
            }
        }
    }

    #[inline]
    pub fn is_flagged(&self, Pos(i, j): Pos) -> bool {
        self.flags
            .get(i as usize)
            .and_then(|a| a.get(j as usize))
            .copied()
            .unwrap_or_default()
    }

    #[inline]
    pub fn call(&self, Pos(i, j): Pos) -> Option<i32> {
        self.call
            .get(i as usize)
            .and_then(|a| a.get(j as usize))
            .copied()
    }
}

impl Grid {
    pub fn spread(
        &self,
        u: &mut [impl IndexMut<usize, Output = i32>],
        v: &mut [impl IndexMut<usize, Output = i32>],
        Pos(x, y): Pos,
        val: i32,
        factor: i32,
    ) {
        let (xu, yu) = (x as usize, y as usize);
        if !self.tile(Pos(x, y)).is_some_and(Tile::is_habitable) {
            return;
        }

        let d = val - u[xu][yu];
        if d > 0 {
            {
                let vv = &mut v[xu][yu];
                *vv = 0.max(*vv + d * factor);
                u[xu][yu] += d;
            }
            for Pos(xd, yd) in Pos::DIRS {
                self.spread(u, v, Pos(x + xd, y + yd), val / 2, factor)
            }
        }
    }

    pub fn even(&self, v: &mut [impl IndexMut<usize, Output = i32>], Pos(x, y): Pos, val: i32) {
        if x < 0
            || x >= self.width as i32
            || y < 0
            || y >= self.height as i32
            || v[x as usize][y as usize] == val
        {
            return;
        }

        v[x as usize][y as usize] = val;
        for Pos(xd, yd) in Pos::DIRS {
            self.even(v, Pos(x + xd, y + yd), val)
        }
    }
}
