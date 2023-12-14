use crate::{
    grid::{HabitLand, Tile},
    Difficulty, Error, FlagGrid, Grid, Player, Pos, FLAG_POWER, MAX_POPULATION,
};

/// Data about each country.
#[derive(Debug)]
pub struct Country {
    player: Player,
    gold: u64,
}

pub const PRICE_VILLAGE: u64 = 160;
pub const PRICE_TOWN: u64 = 240;
pub const PRICE_FORTRESS: u64 = 320;

impl Grid {
    /// Builds a village, upgrades a village to a town,
    /// or upgrades a town to a fortress.
    ///
    /// Returns whether the build was successed.
    pub fn build(&mut self, country: &mut Country, pos: Pos) -> crate::Result<()> {
        let Tile::Habitable { land, .. } = self
            .tile_mut(pos)
            .ok_or(Error::PosOutOfBound(pos))
            .and_then(|t| {
                if t.owner() == country.player {
                    Ok(t)
                } else {
                    Err(Error::NotOwner {
                        operator: country.player,
                        owner: t.owner(),
                        tile: pos,
                    })
                }
            })?
        else {
            return Err(Error::TileNotHabitable(pos));
        };

        let mut l = *land;
        let price = l.upgrade().ok_or(Error::UpgradeTopLevelBuilding)?;
        if country.gold >= price {
            *land = l;
            country.gold -= price;
            Ok(())
        } else {
            Err(Error::InsufficientGold {
                required: price,
                owning: country.gold,
            })
        }
    }

    /// Degrades a city.
    ///
    /// A fortress degrades to a town,
    /// a town degrades to a village,
    /// and a village is destroyed.
    pub fn degrade(&mut self, pos: Pos) -> crate::Result<()> {
        let Tile::Habitable { land, .. } = self.tile_mut(pos).ok_or(Error::PosOutOfBound(pos))?
        else {
            return Err(Error::TileNotHabitable(pos));
        };
        if land.degrade() {
            Ok(())
        } else {
            Err(Error::DegradeGrassLand)
        }
    }
}

#[derive(Debug)]
pub struct King {
    values: Vec<Vec<i32>>,
    player: Player,

    strategy: Strategy,
}

/// Greedy strategy for a [`King`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    None,
    AggrGreedy,
    OneGreedy,
    /// Have more desire to own tiles.
    PersistentGreedy,
    Opportunist,
    /// Have more desire to fortresses,
    /// and less desire to villages than
    /// other strategies.
    Noble,
    /// Have more desire to control mines.
    /// Will never place flags.
    Midas,
}

impl Strategy {
    #[inline]
    const fn habitable_tile_val_addition(self) -> i32 {
        match self {
            Self::PersistentGreedy => 2,
            _ => 1,
        }
    }

    #[inline]
    const fn city_spread_val(self, city: HabitLand) -> i32 {
        match (self, city) {
            (Self::Noble, HabitLand::Fortress) => 32,
            (_, HabitLand::Fortress) => 16,
            (_, HabitLand::Town) => 8,
            (Self::Noble, HabitLand::Village) => 2,
            (_, HabitLand::Village) => 4,
            _ => 0,
        }
    }

    #[inline]
    const fn mine_spread_val(self) -> i32 {
        match self {
            Self::Midas => 8,
            _ => 4,
        }
    }

    #[inline]
    fn process_base(self, val: impl FnOnce() -> i32, base: &mut f32) {
        match self {
            Self::Midas => *base *= (val() + 10) as f32,
            _ => (),
        }
    }
}

impl King {
    /// Creates a new king.
    #[inline]
    pub fn new(player: Player, strat: Strategy, grid: &Grid) -> Self {
        Self {
            values: vec![vec![0; grid.height() as usize]; grid.width() as usize],
            player,
            strategy: strat,
        }
    }

    /// Evaluates the grid.
    ///
    /// Difficulty determines the quality of evaluation.
    pub fn evaluate(&mut self, grid: &Grid, difficulty: Difficulty) {
        self.values.iter_mut().for_each(|a| a.fill(0));
        let mut u = self.values.clone();

        enum Pt {
            Land(HabitLand),
            Mine,
        }

        for (i, arr) in grid.raw_tiles().iter().enumerate() {
            for (j, pt) in arr.iter().enumerate().filter_map(|(j, t)| match t {
                Tile::Habitable { land, .. } => Some((j, Pt::Land(*land))),
                Tile::Mine(_) => Some((j, Pt::Mine)),
                _ => None,
            }) {
                match pt {
                    Pt::Land(l) => {
                        self.values[i][j] += self.strategy.habitable_tile_val_addition();

                        let pos = Pos(i as i32, j as i32);
                        grid.spread(
                            &mut u,
                            &mut self.values,
                            pos,
                            self.strategy.city_spread_val(l),
                            1,
                        );
                        grid.even(&mut u, pos, 0);
                    }
                    Pt::Mine => {
                        for Pos(di, dj) in Pos::DIRS {
                            let pos = Pos(di + i as i32, dj + j as i32);
                            grid.spread(
                                &mut u,
                                &mut self.values,
                                pos,
                                self.strategy.mine_spread_val(),
                                1,
                            );
                            grid.even(&mut u, pos, 0);
                        }
                    }
                }
            }
        }

        // Dumb down king.
        for arr in self.values.iter_mut() {
            for val in arr.iter_mut() {
                match difficulty {
                    Difficulty::Easiest => {
                        *val = *val / 4 + fastrand::i32(..7) - 3;
                    }
                    Difficulty::Easy => {
                        *val = *val / 2 + fastrand::i32(..3) - 1;
                    }
                    _ => (),
                }
            }
        }
    }

    /// Build cities and returns whether something
    /// was built.
    ///
    /// The strategy is same for all AIs.
    pub fn build(&self, grid: &mut Grid, country: &mut Country) -> bool {
        assert_eq!(self.player, country.player);

        let mut v_best = 0.0;
        let (mut i_best, mut j_best) = (0, 0);

        for (i, arr) in grid.raw_tiles().iter().enumerate() {
            for (j, tile) in arr.iter().enumerate() {
                let mut ok = false;
                if tile.owner() == self.player && tile.is_habitable() {
                    ok = true;
                    for Pos(di, dj) in Pos::DIRS {
                        if let Some(tile) = grid
                            .tile(Pos(i as i32 + di, j as i32 + dj))
                            .filter(|t| t.is_habitable())
                        {
                            ok = ok && tile.owner() == self.player;
                        }
                    }
                }

                if let Tile::Habitable { units, land, .. } = tile {
                    let pl = self.player.0 as usize;
                    let army = units[pl];

                    let mut base = match land {
                        HabitLand::Grassland => 1.0,
                        HabitLand::Village => 8.0,
                        HabitLand::Town => 32.0,
                        _ => 0.0,
                    };
                    self.strategy.process_base(|| self.values[i][j], &mut base);
                    let v = if ok {
                        base * (MAX_POPULATION - army) as f32
                    } else {
                        0.0
                    };

                    if v > v_best {
                        i_best = i;
                        j_best = j;
                        v_best = v;
                    }
                }
            }
        }

        if v_best > 0.0 {
            grid.build(country, Pos(i_best as i32, j_best as i32))
                .is_ok()
        } else {
            false
        }
    }

    /// Place flags based on the strategy.
    #[inline]
    pub fn place_flags(&self, grid: &Grid, fg: &mut FlagGrid) {
        macro_rules! action {
            ($f:ident) => {
                $f(self, grid, fg)
            };
        }
        match self.strategy {
            Strategy::AggrGreedy => action!(action_aggr_greedy),
            Strategy::OneGreedy => action!(action_one_greedy),
            Strategy::PersistentGreedy => action!(action_persistent_greedy),
            Strategy::Opportunist => action!(action_opportunist),
            Strategy::Noble => action!(action_noble),
            _ => (),
        }
    }
}

fn action_aggr_greedy(king: &King, grid: &Grid, fg: &mut FlagGrid) {
    for (i, (arr_g, arr_k)) in grid.raw_tiles().iter().zip(&king.values).enumerate() {
        for (j, (tile, val)) in arr_g.iter().zip(arr_k.iter().copied()).enumerate() {
            if let Tile::Habitable { units, .. } = tile {
                let pos = Pos(i as i32, j as i32);

                let pl = king.player.0 as usize;
                let army = units[pl];
                let enemy =
                    units[..pl].into_iter().sum::<u16>() + units[pl + 1..].into_iter().sum::<u16>();
                if (val * (2 * enemy as i32 - army as i32)) as f32 * (army as f32).powf(0.5)
                    > 5000.0
                {
                    fg.add(grid, pos, FLAG_POWER);
                } else {
                    fg.remove(grid, pos, FLAG_POWER);
                }
            }
        }
    }
}

fn action_one_greedy(king: &King, grid: &Grid, fg: &mut FlagGrid) {
    let mut v_best = -1.0;
    let mut best_pos = Pos(0, 0);
    for (i, (arr_g, arr_k)) in grid.raw_tiles().iter().zip(&king.values).enumerate() {
        for (j, (tile, val)) in arr_g.iter().zip(arr_k.iter().copied()).enumerate() {
            let pos = Pos(i as i32, j as i32);
            if fg.is_flagged(pos) {
                fg.remove(grid, pos, FLAG_POWER);
            }

            if let Tile::Habitable { units, .. } = tile {
                let pl = king.player.0 as usize;
                let army = units[pl];
                let enemy =
                    units[..pl].into_iter().sum::<u16>() + units[pl + 1..].into_iter().sum::<u16>();
                let v = (val * (5 * enemy as i32 - army as i32)) as f32 * (army as f32).powf(0.5);
                if v > v_best && v > 5000.0 {
                    v_best = v;
                    best_pos = pos;
                }
            }
        }
    }

    if v_best > 0.0 {
        fg.add(grid, best_pos, FLAG_POWER)
    }
}

fn action_persistent_greedy(king: &King, grid: &Grid, fg: &mut FlagGrid) {
    for (i, (arr_g, arr_k)) in grid.raw_tiles().iter().zip(&king.values).enumerate() {
        for (j, (tile, val)) in arr_g.iter().zip(arr_k.iter().copied()).enumerate() {
            let pos = Pos(i as i32, j as i32);
            if let Tile::Habitable { units, .. } = tile {
                let pl = king.player.0 as usize;
                let army = units[pl];
                let enemy =
                    units[..pl].into_iter().sum::<u16>() + units[pl + 1..].into_iter().sum::<u16>();
                let v = (val as f32 * (2.5 * enemy as f32 - army as f32) * (army as f32).powf(0.7))
                    .max(
                        (val * (MAX_POPULATION as i32 - enemy as i32 + army as i32)) as f32
                            * (army as f32).powf(0.7),
                    );

                if fg.is_flagged(pos) && v < 1000.0 {
                    fg.remove(grid, pos, FLAG_POWER);
                } else if v > 9000.0 {
                    fg.add(grid, pos, FLAG_POWER);
                }
            }
        }
    }
}

fn action_opportunist(king: &King, grid: &Grid, fg: &mut FlagGrid) {
    for (i, (arr_g, arr_k)) in grid.raw_tiles().iter().zip(&king.values).enumerate() {
        for (j, (tile, val)) in arr_g.iter().zip(arr_k.iter().copied()).enumerate() {
            if let Tile::Habitable { units, .. } = tile {
                let pos = Pos(i as i32, j as i32);

                let pl = king.player.0 as usize;
                let army = units[pl];
                let enemy =
                    units[..pl].into_iter().sum::<u16>() + units[pl + 1..].into_iter().sum::<u16>();
                if enemy > army
                    && (val * (MAX_POPULATION as i32 - enemy as i32 + army as i32)) as f32
                        * (army as f32).powf(0.5)
                        > 7000.0
                {
                    fg.add(grid, pos, FLAG_POWER);
                } else {
                    fg.remove(grid, pos, FLAG_POWER);
                }
            }
        }
    }
}

fn action_noble(king: &King, grid: &Grid, fg: &mut FlagGrid) {
    const MAX_PRIORITY: usize = 32;

    struct PosVal<const N: usize> {
        locs: [Pos; N],
        vals: [i32; N],
    }

    impl<const N: usize> PosVal<N> {
        const NO_LOC: Pos = Pos(-1, -1);

        #[inline]
        const fn new() -> Self {
            Self {
                locs: [Self::NO_LOC; N],
                vals: [-1; N],
            }
        }

        fn insert(&mut self, lx: Pos, vx: i32) {
            let i = self
                .vals
                .into_iter()
                .position(|val| val < vx)
                .unwrap_or(N.min(MAX_PRIORITY));
            if i < MAX_PRIORITY {
                let mut locs = self.locs;
                let mut vals = self.vals;

                {
                    let mut li = self.locs[i + 1..].into_iter().copied().rev();
                    let mut vi = self.vals.into_iter().rev();

                    li.next();
                    vi.next();

                    for ((loc, val), (l, v)) in locs
                        .iter_mut()
                        .rev()
                        .zip(vals.iter_mut().rev())
                        .zip(li.zip(vi))
                    {
                        *loc = l;
                        *val = v;
                    }
                }

                locs[i] = lx;
                vals[i] = vx;

                self.locs = locs;
                self.vals = vals;
            }
        }
    }

    const LEN: usize = 5;

    let mut pos_val: PosVal<LEN> = PosVal::new();
    for (i, (arr_g, arr_k)) in grid.raw_tiles().iter().zip(&king.values).enumerate() {
        for (j, (tile, val)) in arr_g.iter().zip(arr_k.iter().copied()).enumerate() {
            if let Tile::Habitable { units, .. } = tile {
                let pos = Pos(i as i32, j as i32);

                let pl = king.player.0 as usize;
                let army = units[pl];
                let enemy =
                    units[..pl].into_iter().sum::<u16>() + units[pl + 1..].into_iter().sum::<u16>();
                let v = (val * (MAX_POPULATION as i32 - enemy as i32 + army as i32)) as f32
                    * (army as f32).powf(0.5);

                if enemy > army && v > 7000.0 {
                    pos_val.insert(pos, v as i32)
                }
            }
        }
    }

    pos_val
        .locs
        .into_iter()
        .zip(pos_val.vals)
        .take_while(|(_, v)| *v > 0)
        .map(|(p, _)| p)
        .for_each(|p| fg.add(grid, p, FLAG_POWER));
}
