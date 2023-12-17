use crate::{
    grid::{Stencil, Tile, MAX_AVLBL_LOCS},
    Country, Difficulty, FlagGrid, Grid, King, Player, Pos, Speed, Strategy, MAX_HEIGHT,
    MAX_PLAYERS, MAX_WIDTH,
};

#[derive(Debug)]
pub struct UI {
    cursor: Pos,
    /// Number of tiles to skip in the beginning of
    /// every line.
    xskip: u32,
    /// Total max number of tiles in horizontal direction.
    xlen: u32,
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
}

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
}

pub struct MultiplayerOpts {
    pub multiplayer: bool,
    pub server: bool,

    pub client_port: u32,
    pub server_addr: String,
    pub server_port: u32,

    pub clients_num: usize,
}

/// Game state.
pub struct State {
    /// The map grid.
    grid: Grid,
    /// The array of flag grids of each players.
    fgs: [FlagGrid; MAX_PLAYERS],
    /// AI opponents.
    kings: Vec<King>,

    timeline: Timeline,
    show_timeline: bool,

    countries: [Country; MAX_PLAYERS],

    time: u64,
    /// The map seed.
    seed: u64,
    /// Player id of the human controlled player.
    controlled: Player,

    conditions: Option<u32>,
    inequality: Option<u32>,

    speed: Speed,
    prev_speed: Speed,
    difficulty: Difficulty,
}

impl State {
    pub fn new(b_opt: BasicOpts, mp_opt: MultiplayerOpts) -> crate::Result<Self> {
        let width = b_opt.width.min(MAX_WIDTH);
        let height = b_opt.height.min(MAX_HEIGHT);

        const PLAYERS: usize = 7;
        let time = (1850 + fastrand::u64(..100)) * 360 + fastrand::u64(..360);

        let mut all_players = [Player::default(); PLAYERS];
        all_players
            .iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v = Player(i as u32 + 1));
        let comp_players = all_players[mp_opt.clients_num..].to_vec();
        let ui_players = all_players[..mp_opt.clients_num].to_vec();

        let mut kings: Vec<King> = (0..ui_players.len())
            .map(|i| {
                King::new(
                    Player(i as u32 + 1),
                    match i + mp_opt.clients_num {
                        1 => Strategy::Opportunist,
                        2 => Strategy::OneGreedy,
                        3 => Strategy::None,
                        4 => Strategy::AggrGreedy,
                        5 => Strategy::Noble,
                        6 => Strategy::PersistentGreedy,
                        _ => Default::default(),
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
            .for_each(|k| k.evaluate(&grid, b_opt.difficulty));

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
}
