use curseofrust::{grid::Tile, state::State, Player};

use crate::{S2CData, TileClass};

pub fn apply_s2c_msg(state: &mut State, data: S2CData) -> curseofrust::Result<()> {
    if u32::from_be(data.time) as u64 <= state.time {
        return Err(curseofrust::Error::DeprecatedMsg {
            time: u32::from_be(data.time),
        });
    }

    state.time = u32::from_be(data.time) as u64;
    for (p1, p2) in state
        .countries
        .iter_mut()
        .map(|c| &mut c.gold)
        .zip(data.gold)
    {
        *p1 = u32::from_be(p2) as u64;
    }
    for fg in &mut state.fgs {
        fg.width = data.width as u32;
        fg.height = data.height as u32;
    }
    state.controlled = Player(data.player as u32);
    for (x, arr) in state.grid.raw_tiles_mut().iter_mut().enumerate() {
        for (y, tile) in arr.iter_mut().enumerate() {
            let Some(target) = data
                .tile
                .get(x)
                .and_then(|a| a.get(y))
                .copied()
                .and_then(|a| TileClass::try_from(a).ok())
            else {
                // This make sure that the (x, y) indexes are valid for the data message.
                continue;
            };
            let mut t: Tile = target.into();
            let owner = data.owner[x][y];
            t.set_owner(Player(owner as u32));
            if let Some(unit) = t.units_mut().and_then(|us| us.get_mut(owner as usize)) {
                *unit = u16::from_be(data.pop[x][y]);
            }
            *tile = t;

            for (p, fg) in state.fgs.iter_mut().enumerate() {
                fg.call[x][y] = 0;
                fg.flags[x][y] = data.flag[x][y] & (1 << p) != 0;
            }
        }
    }

    Ok(())
}
