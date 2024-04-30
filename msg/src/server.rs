//! Server utils.

use std::net::SocketAddr;

use curseofrust::{state::State, Player, Pos, FLAG_POWER, MAX_HEIGHT, MAX_WIDTH};

use crate::{
    client_msg::*, C2SData, S2CData, TileClass, __S2C_PAD_0_LEN, __S2C_PAD_1_LEN, __S2C_PAD_2_LEN,
};

#[derive(Debug, Clone)]
pub struct ClientRecord {
    /// Player of the client.
    pub player: Player,
    pub id: u32,
    pub name: String,
    /// Socket address of the client.
    pub addr: SocketAddr,
}

/// Mode of a server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    /// Waiting for clients.
    Lobby,
    /// Playing
    Play,
}

impl S2CData {
    /// Creates a new `S2CData` from the given `State`.
    pub fn new(player: Player, state: &State) -> Self {
        let mut flag = [[0u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize];
        for (x, arr) in flag.iter_mut().enumerate() {
            for (y, flag) in arr.iter_mut().enumerate() {
                for (p, f) in state.fgs.iter().enumerate() {
                    if f.is_flagged(Pos(x as i32, y as i32)) {
                        *flag |= 1 << p;
                    }
                }
            }
        }

        let mut owner = [[0u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize];
        let mut pop = [[0u16; MAX_HEIGHT as usize]; MAX_WIDTH as usize];
        let mut tile = [[0u8; MAX_HEIGHT as usize]; MAX_WIDTH as usize];
        for (x, arr) in state.grid.raw_tiles().iter().enumerate() {
            for (y, t) in arr.iter().enumerate() {
                let ow = t.owner().0;
                owner[x][y] = ow as u8;
                pop[x][y] = t.units()[ow as usize].to_be();
                tile[x][y] = TileClass::from(t) as u8;
            }
        }

        S2CData {
            player: player.0 as u8,
            pause_request: 0,
            gold: state.countries.each_ref().map(|c| (c.gold as u32).to_be()),
            time: (state.time as u32).to_be(),
            width: state.grid.width() as u8,
            height: state.grid.height() as u8,
            flag,
            owner,
            pop,
            tile,
            __pad0: [0; __S2C_PAD_0_LEN],
            __pad1: [0; __S2C_PAD_1_LEN],
            __pad2: [0; __S2C_PAD_2_LEN],
        }
    }

    /// Sets the player of the data.
    #[inline]
    pub fn set_player(&mut self, player: Player) {
        self.player = player.0 as u8;
    }
}

pub fn apply_c2s_msg(
    state: &mut State,
    player: Player,
    msg: u8,
    data: C2SData,
) -> curseofrust::Result<()> {
    let pl = player.0 as usize;
    let pos = Pos(data.x as i32, data.y as i32);

    match msg {
        BUILD => {
            return state.grid.build(
                state
                    .countries
                    .get_mut(pl)
                    .ok_or(curseofrust::Error::PlayerNotFound(player))?,
                pos,
            )
        }
        FLAG_ON => state
            .fgs
            .get_mut(pl)
            .ok_or(curseofrust::Error::PlayerNotFound(player))?
            .add(&state.grid, pos, FLAG_POWER),
        FLAG_OFF => state
            .fgs
            .get_mut(pl)
            .ok_or(curseofrust::Error::PlayerNotFound(player))?
            .remove(&state.grid, pos, FLAG_POWER),
        FLAG_OFF_ALL => state
            .fgs
            .get_mut(pl)
            .ok_or(curseofrust::Error::PlayerNotFound(player))?
            .remove_with_prob(&state.grid, 1.0),
        FLAG_OFF_HALF => state
            .fgs
            .get_mut(pl)
            .ok_or(curseofrust::Error::PlayerNotFound(player))?
            .remove_with_prob(&state.grid, 0.5),
        _ => {}
    }
    Ok(())
}
