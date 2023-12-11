macro_rules! in_segment {
    ($x:expr, $l:expr, $r:expr) => {
        if $x < $l {
            $l
        } else if $x > $r {
            $r
        } else {
            $x
        }
    };
}

mod grid;
mod state;

const MAX_WIDTH: u32 = 40;
const MAX_HEIGHT: u32 = 29;

const MAX_PLAYERS: usize = 8;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
struct Player(u32);

impl Player {
    const NEUTRAL: Self = Self(0);

    #[inline]
    fn is_neutral(self) -> bool {
        self == Self::NEUTRAL
    }
}

fn main() {
    println!("Hello, world!");
}
