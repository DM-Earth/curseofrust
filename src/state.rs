use std::net::SocketAddr;

use crate::{
    grid::{HabitLand, Stencil, Tile, MAX_AVLBL_LOCS},
    Country, Difficulty, FlagGrid, Grid, King, Player, Pos, Speed, Strategy, MAX_HEIGHT,
    MAX_PLAYERS, MAX_POPULATION, MAX_WIDTH,
};

#[derive(Debug)]
#[non_exhaustive]
pub struct UI {
    pub cursor: Pos,
    /// Number of tiles to skip in the beginning of
    /// every line.
    pub xskip: u16,
    /// Total max number of tiles in horizontal direction.
    pub xlen: u16,
}

pub struct Timeline {
    data: [[f32; Self::MAX_MARKS]; MAX_PLAYERS],
    /// Time when data was recorded.
    time: [u64; Self::MAX_MARKS],

    /// The most recently updated time mark.
    /// It can be used as index in two other fields.
    ///
    /// `0 <= mark < MAX_MAKRS`.
    mark: usize,
}

impl Timeline {
    pub const MAX_MARKS: usize = 72;

    pub(crate) fn update(&mut self, time: u64, grid: &Grid) {
        if self.mark + 1 < Self::MAX_MARKS {
            self.mark += 1;
        } else {
            for i in 0..Self::MAX_MARKS {
                self.time[i] = self.time[i + 1];
                for p in 0..MAX_PLAYERS {
                    self.data[p][i] = self.data[p][i + 1];
                }
            }
        }

        self.time[self.mark] = time;
        for p in 0..MAX_PLAYERS {
            self.data[p][self.mark] = grid
                .raw_tiles()
                .iter()
                .map(|a| a.iter().map(|t| t.units()[p]).sum::<u16>() as u32)
                .sum::<u32>() as f32;
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct BasicOpts {
    pub keep_random: bool,
    pub difficulty: Difficulty,
    pub speed: Speed,

    pub width: u32,
    pub height: u32,
    pub locations: usize,
    pub seed: u64,
    pub conditions: Option<u32>,
    pub timeline: bool,

    pub inequality: Option<u32>,
    pub shape: Stencil,

    pub clients: usize,
}

impl Default for BasicOpts {
    fn default() -> Self {
        Self {
            keep_random: false,
            difficulty: Default::default(),
            speed: Default::default(),
            width: 21,
            height: 21,
            locations: Stencil::default().max_locs(),
            seed: fastrand::u64(..),
            conditions: Default::default(),
            timeline: false,
            inequality: Default::default(),
            shape: Default::default(),
            clients: 1,
        }
    }
}

#[derive(Default, Debug)]
pub enum MultiplayerOpts {
    Server {
        port: u16,
    },
    Client {
        server: SocketAddr,
        port: u16,
    },
    #[default]
    None,
}

/// Game state.
#[non_exhaustive]
pub struct State {
    /// The map grid.
    pub grid: Grid,
    /// The array of flag grids of each players.
    pub fgs: [FlagGrid; MAX_PLAYERS],
    /// AI opponents.
    pub kings: Vec<King>,

    pub timeline: Timeline,
    pub show_timeline: bool,

    pub countries: [Country; MAX_PLAYERS],

    pub time: u64,
    /// The map seed.
    pub seed: u64,
    /// Player id of the human controlled player.
    pub controlled: Player,

    pub conditions: Option<u32>,
    pub inequality: Option<u32>,

    pub speed: Speed,
    pub prev_speed: Speed,
    pub difficulty: Difficulty,
}

macro_rules! rnd_round {
    ($x:expr) => {{
        let mut i = $x as i32;
        if fastrand::f32() < ($x - i as f32) {
            i += 1;
        }
        i
    }};
}

impl State {
    pub fn new(b_opt: BasicOpts) -> crate::Result<Self> {
        let width = b_opt.width.min(match b_opt.shape {
            Stencil::Rect => MAX_WIDTH + 10,
            _ => MAX_WIDTH,
        });
        let height = b_opt.height.min(MAX_HEIGHT);

        const PLAYERS: usize = 7;
        let time = (1850 + fastrand::u64(..100)) * 360 + fastrand::u64(..360);

        let mut all_players = [Player::default(); PLAYERS];
        all_players
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v = Player(i as u32 + 1));
        let comp_players = all_players[b_opt.clients..].to_vec();
        let ui_players = all_players[..b_opt.clients].to_vec();

        let mut kings: Vec<King> = (b_opt.clients..7)
            .map(|i| {
                King::new(
                    Player(i as u32 + 1),
                    match i as isize - b_opt.clients as isize {
                        0 => Strategy::Opportunist,
                        1 => Strategy::OneGreedy,
                        2 => Strategy::Midas,
                        3 => Strategy::AggrGreedy,
                        4 => Strategy::Noble,
                        5 => Strategy::PersistentGreedy,
                        _ => unreachable!(),
                    },
                    width,
                    height,
                )
            })
            .collect();

        fastrand::seed(b_opt.seed);
        let mut grid = Grid::new(b_opt.width, b_opt.height);

        // Map generation
        loop {
            grid.raw_tiles_mut()
                .iter_mut()
                .for_each(|a| a.fill_with(Tile::new));
            let mut loc_arr = [Pos(0, 0); MAX_AVLBL_LOCS];
            let avlbl_loc_num = b_opt.shape.max_locs();
            b_opt
                .shape
                .apply(&mut grid, 2, &mut loc_arr[..avlbl_loc_num]);

            if grid
                .conflict(crate::grid::ConflictDescriptor {
                    locs: &mut loc_arr[..avlbl_loc_num],
                    locs_num: b_opt.locations,
                    players: &comp_players,
                    ui_players: &ui_players,
                    conditions: b_opt.conditions,
                    ineq: b_opt.inequality,
                })
                .is_ok_and(|_| grid.is_connected())
            {
                break;
            }
        }

        let fgs = [0; MAX_PLAYERS].map(|_| FlagGrid::new(width, height));
        let mut countries = [0; MAX_PLAYERS];
        countries.iter_mut().enumerate().for_each(|(i, c)| *c = i);
        let countries = countries.map(|c| Country::from(Player(c as u32)));

        kings
            .iter_mut()
            .for_each(|k| k.evaluate_map(&grid, b_opt.difficulty));

        let timeline = Timeline {
            data: [[0.0; Timeline::MAX_MARKS]; MAX_PLAYERS],
            time: [time; Timeline::MAX_MARKS],
            mark: 0,
        };

        Ok(Self {
            grid,
            fgs,
            kings,
            timeline,
            show_timeline: b_opt.timeline,
            countries,
            time,
            seed: fastrand::get_seed(),
            controlled: Player(1),
            conditions: b_opt.conditions,
            inequality: b_opt.inequality,
            speed: b_opt.speed,
            prev_speed: b_opt.speed,
            difficulty: b_opt.difficulty,
        })
    }

    /// Kings build cities and place flags.
    pub fn kings_move(&mut self) {
        let mut ev = false;
        for king in &self.kings {
            let Player(pl) = king.player();
            king.place_flags(&self.grid, &mut self.fgs[pl as usize]);
            let res = king.build(&mut self.grid, &mut self.countries[pl as usize]);
            ev = ev || res;
        }
        if ev {
            for king in &mut self.kings {
                king.evaluate_map(&self.grid, self.difficulty);
            }
        }
    }

    /// Performs one step of the game simulation.
    pub fn simulate(&mut self) {
        self.time += 1;
        let mut need_to_reeval = false;

        for i in 0..self.grid.width() {
            for j in 0..self.grid.height() {
                // Mines ownership
                if self
                    .grid
                    .tile(Pos(i as i32, j as i32))
                    .is_some_and(|t| matches!(t, Tile::Mine(_)))
                {
                    let mut owner = Some(Player::NEUTRAL);
                    for dir in Pos::DIRS {
                        if let Some(pl) = self
                            .grid
                            .tile(Pos(i as i32 + dir.0, j as i32 + dir.1))
                            .and_then(|t| {
                                if t.is_habitable() {
                                    Some(t.owner())
                                } else {
                                    None
                                }
                            })
                        {
                            if owner == Some(Player::NEUTRAL) {
                                owner = Some(pl);
                            } else if owner != Some(pl) && !pl.is_neutral() {
                                owner = None;
                            }
                        }
                    }
                    let t = self.grid.tile_mut(Pos(i as i32, j as i32)).unwrap();
                    if let Some(owner) = owner {
                        t.set_owner(owner);
                        if !owner.is_neutral() {
                            self.countries[owner.0 as usize].gold += 1;
                        }
                    } else {
                        t.set_owner(Player::NEUTRAL);
                    }
                }

                if let Tile::Habitable {
                    ref mut units,
                    owner,
                    land,
                } = self.grid.raw_tiles_mut()[i as usize][j as usize]
                {
                    let my_pops = *units;
                    let total_pop = my_pops[0];
                    let enemy_pops = my_pops.map(|p| total_pop - p);

                    let mut defender_dmg = 0;
                    for (p, (my_pop, enemy_pop)) in my_pops.into_iter().zip(enemy_pops).enumerate()
                    {
                        if p == 0 {
                            continue;
                        }
                        let mut dmg = 0;
                        if total_pop != 0 {
                            dmg = rnd_round!(enemy_pop as f32 * my_pop as f32 / total_pop as f32);
                        }
                        units[p] = (my_pop as i32 - dmg).max(0) as u16;
                        if owner == Player(p as u32) {
                            defender_dmg = dmg;
                        }
                    }

                    const ATTACK: f32 = 0.1;

                    // Burning cities
                    if defender_dmg as f32 > 2.0 * MAX_POPULATION as f32 * ATTACK
                        && land != HabitLand::Grassland
                        && fastrand::bool()
                    {
                        need_to_reeval = true;
                        let _ = self.grid.degrade(Pos(i as i32, j as i32));
                    }

                    let Tile::Habitable {
                        ref mut units,
                        ref mut owner,
                        land,
                    } = self.grid.raw_tiles_mut()[i as usize][j as usize]
                    else {
                        unreachable!()
                    };

                    // Determine ownership
                    *owner = Player::NEUTRAL;
                    let mut o_unit = 0;
                    for (pr, &u) in units[1..].iter().enumerate() {
                        if u > o_unit {
                            *owner = Player((pr + 1) as u32);
                            o_unit = u;
                        }
                    }

                    // Population growth
                    if land != HabitLand::Grassland {
                        let pop = units[owner.0 as usize];
                        let fnpop = pop as f32 * land.growth();
                        let npop = (rnd_round!(fnpop) as u16).min(MAX_POPULATION);
                        units[owner.0 as usize] = npop;
                    }
                }
            }
        }

        let i_start;
        let j_start;
        let i_end;
        let j_end;
        let i_inc;
        let j_inc;

        if fastrand::u8(..2) == 0 {
            i_start = 0;
            i_end = self.grid.width() as i32;
            i_inc = 1;
        } else {
            i_start = self.grid.width() as i32 - 1;
            i_end = -1;
            i_inc = -1;
        }

        if fastrand::u8(..2) == 0 {
            j_start = 0;
            j_end = self.grid.height() as i32;
            j_inc = 1;
        } else {
            j_start = self.grid.height() as i32 - 1;
            j_end = -1;
            j_inc = -1;
        }

        let mut i = i_start;
        while i != i_end {
            let mut j = j_start;
            while j != j_end {
                for p in 1..MAX_PLAYERS {
                    let Some(tile) = self.grid.tile(Pos(i, j)) else {
                        continue;
                    };
                    let initial_pop = tile.units()[p];
                    let k_shift = fastrand::usize(..6);
                    let fg = &self.fgs[p];

                    for k in 0..6 {
                        let tile = self.grid.tile(Pos(i, j)).unwrap();
                        let dir = Pos::DIRS[(k + k_shift) % 6];
                        let pos = Pos(i + dir.0 as i32, j + dir.1 as i32);
                        if let Some(Tile::Habitable { units, .. }) = self.grid.tile(pos) {
                            let pop = tile.units()[p];
                            let dcall = (fg.call(pos).unwrap_or_default()
                                - fg.call(Pos(i, j)).unwrap_or_default())
                            .max(0);

                            const MOVE: f32 = 0.05;
                            const CALL_MOVE: f32 = 0.10;
                            let dpop = rnd_round!(
                                MOVE * initial_pop as f32
                                    + CALL_MOVE * dcall as f32 * initial_pop as f32
                            )
                            .min(pop as i32)
                            .min((MAX_POPULATION - units[p]) as i32);

                            let Some(Tile::Habitable { units, .. }) = self.grid.tile_mut(pos)
                            else {
                                unreachable!()
                            };
                            units[p] = (units[p] as i32 + dpop).max(0) as u16;
                            if let Some(Tile::Habitable { units, .. }) =
                                self.grid.tile_mut(Pos(i, j))
                            {
                                units[p] = (units[p] as i32 - dpop).max(0) as u16;
                            }
                        }
                    }
                }
                j += j_inc;
            }
            i += i_inc;
        }

        // Determine ownership again
        for arr in self.grid.raw_tiles_mut() {
            for tile in arr {
                let Tile::Habitable { units, owner, .. } = tile else {
                    continue;
                };
                *owner = Player::NEUTRAL;
                units[0] = units[1..].iter().sum::<u16>();
                let mut o_unit = 0;
                for (pr, &u) in units[1..].iter().enumerate() {
                    if u > o_unit {
                        *owner = Player((pr + 1) as u32);
                        o_unit = u;
                    }
                }
            }
        }

        // Kings re-evaluate the map
        if need_to_reeval {
            self.kings
                .iter_mut()
                .for_each(|t| t.evaluate_map(&self.grid, self.difficulty));
        }

        // Give gold to AI on hard difficulties
        let add_gold = match self.difficulty {
            Difficulty::Hard => 1,
            Difficulty::Hardest => 2,
            _ => 0,
        };
        if add_gold > 0 {
            for i in 1..MAX_PLAYERS {
                let pl = Player(i as u32);
                let c = &mut self.countries[i];
                if pl != self.controlled && c.gold > 0 {
                    c.gold += add_gold;
                }
            }
        }
    }

    #[inline]
    pub fn update_timeline(&mut self) {
        self.timeline.update(self.time, &self.grid)
    }
}

impl UI {
    /// Creates a new `UI` from given [`State`].
    ///
    /// Cursor position and `xskip` will be computed.
    pub fn new(state: &State) -> Self {
        let mut cursor = (
            state.grid.width() as usize / 2,
            state.grid.height() as usize / 2,
        );
        {
            let p = state.controlled.0 as usize;
            let mut pointing_pop = 0;

            for (i, arr) in state.grid.raw_tiles().iter().enumerate() {
                for (j, pop) in arr.iter().enumerate().filter_map(|(j, t)| {
                    if let Tile::Habitable { units, .. } = t {
                        Some((j, units[p]))
                    } else {
                        None
                    }
                }) {
                    if pop > pointing_pop {
                        pointing_pop = pop;
                        cursor = (i, j);
                    }
                }
            }
        }
        let cursor = Pos(cursor.0 as i32, cursor.1 as i32);

        let mut xskip_x2 = MAX_WIDTH as usize * 2 + 1;
        let mut xrightmost_x2 = 0;
        for (i, arr) in state.grid.raw_tiles().iter().enumerate() {
            for j in arr
                .iter()
                .enumerate()
                .filter_map(|(j, t)| if t.is_visible() { Some(j) } else { None })
            {
                let x = i * 2 + j;
                xskip_x2 = xskip_x2.min(x);
                xrightmost_x2 = xrightmost_x2.max(x);
            }
        }

        Self {
            cursor,
            xskip: (xskip_x2 as u16 + 1) / 2,
            xlen: (xrightmost_x2 as u16 + 1) / 2 - xskip_x2 as u16 / 2,
        }
    }

    /// Change the cursor position by the given position.
    /// Adjust if necessary.
    pub fn adjust_cursor(&mut self, state: &State, pos: Pos) {
        let pos = Pos(
            in_segment!(pos.0, 0, state.grid.width() as i32 - 1),
            in_segment!(pos.1, 0, state.grid.height() as i32 - 1),
        );

        if state.grid.tile(pos).is_some_and(Tile::is_visible) {
            self.cursor = pos;
        } else if state
            .grid
            .tile(Pos(self.cursor.0, pos.1))
            .is_some_and(Tile::is_visible)
        {
            self.cursor.1 = pos.1;
        } else {
            let i = in_segment!(pos.0 - 1, 0, state.grid.width() as i32 - 1);
            if state.grid.tile(Pos(i, pos.1)).is_some_and(Tile::is_visible) {
                self.cursor = Pos(i, pos.1)
            } else {
                let i = in_segment!(pos.0 + 1, 0, state.grid.width() as i32 - 1);
                if state.grid.tile(Pos(i, pos.1)).is_some_and(Tile::is_visible) {
                    self.cursor = Pos(i, pos.1)
                }
            }
        }
    }
}
