mod grid;
mod state;

const MAX_WIDTH: u32 = 40;
const MAX_HEIGHT: u32 = 29;

const MAX_PLAYERS: usize = 8;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
enum Player {
    #[default]
    Neutral,
    Id(u32),
}

impl From<Player> for u32 {
    #[inline]
    fn from(value: Player) -> Self {
        match value {
            Player::Neutral => 0,
            Player::Id(id) => id,
        }
    }
}

impl From<u32> for Player {
    #[inline]
    fn from(value: u32) -> Self {
        if value == 0 {
            Self::Neutral
        } else {
            Self::Id(value)
        }
    }
}

fn main() {
    println!("Hello, world!");
}
