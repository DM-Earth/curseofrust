//! Output functions for the console.

use std::io::Write;

use crossterm::{
    queue,
    style::{self, Color, ContentStyle, StyledContent},
};
use curseofrust::{Player, Pos};

use crate::State;

const MOUNTAIN: &'static str = "/\\^";
const MINE: &'static str = "/$\\";
const VILLAGE: &'static str = " n ";
const TOWN: &'static str = "i=i";
const FORTRESS: &'static str = "W=W";

fn player_style(player: Player) -> ContentStyle {
    ContentStyle {
        foreground_color: Some(player_color(player)),
        attributes: if player.is_neutral() {
            Default::default()
        } else {
            style::Attribute::Bold.into()
        },
        ..Default::default()
    }
}

#[inline]
fn player_color(player: Player) -> Color {
    match player {
        Player::NEUTRAL => Color::Yellow,
        Player(1) => Color::Green,
        Player(2) => Color::Blue,
        Player(3) => Color::Red,
        Player(4) => Color::Yellow,
        Player(5) => Color::Magenta,
        Player(6) => Color::Cyan,
        Player(7) => Color::Black,
        _ => Color::Reset,
    }
}

#[inline]
fn pop_to_symbol(pop: u16) -> &'static str {
    match pop {
        0 => "   ",
        1..=3 => " . ",
        4..=6 => ".. ",
        7..=12 => "...",
        13..=25 => " : ",
        26..=50 => ".: ",
        51..=100 => ".:.",
        101..=200 => " ::",
        201..=400 => ".::",
        401.. => ":::",
    }
}

pub(crate) fn draw_grid<W: Write>(st: &mut State<W>) -> Result<(), std::io::Error> {
    for y in 0..st.s.grid.height() {
        for _ in 0..st.ui.xskip {
            queue!(st.out, style::Print("    "))?;
        }

        for x in 0..st.s.grid.width() {
            let pos = Pos(x as i32, y as i32);
            let Some(tile) = st.s.grid.tile(pos) else {
                break;
            };
            macro_rules! cursor {
                () => {
                    let l_sym = if pos == st.ui.cursor {
                        '('
                    } else if Pos(x as i32 - 1, y as i32) == st.ui.cursor {
                        ')'
                    } else {
                        ' '
                    };
                    queue!(st.out, style::Print(l_sym))?;
                };
            }
            match tile {
                curseofrust::grid::Tile::Void => {
                    cursor!();
                    queue!(st.out, style::Print("    "))?;
                }
                curseofrust::grid::Tile::Mountain => {
                    cursor!();
                    queue! {
                        st.out,
                        style::PrintStyledContent(StyledContent::new(
                            ContentStyle {
                                foreground_color: Some(Color::Green),
                                ..Default::default()
                            },
                            MOUNTAIN,
                        ))
                    }?;
                }
                curseofrust::grid::Tile::Mine(owner) => {
                    cursor!();
                    queue!(
                        st.out,
                        style::PrintStyledContent(StyledContent::new(
                            ContentStyle {
                                foreground_color: Some(Color::Green),
                                ..Default::default()
                            },
                            &MINE[0..1],
                        )),
                        style::PrintStyledContent(StyledContent::new(
                            player_style(*owner),
                            &MINE[1..2],
                        )),
                        style::PrintStyledContent(StyledContent::new(
                            ContentStyle {
                                foreground_color: Some(Color::Green),
                                ..Default::default()
                            },
                            &MINE[2..3],
                        )),
                    )?;
                }
                curseofrust::grid::Tile::Habitable { land, units, owner } => {
                    cursor!();

                    let symbol = match land {
                        curseofrust::grid::HabitLand::Grassland => {
                            pop_to_symbol(units.iter().sum())
                        }
                        curseofrust::grid::HabitLand::Village => VILLAGE,
                        curseofrust::grid::HabitLand::Town => TOWN,
                        curseofrust::grid::HabitLand::Fortress => FORTRESS,
                    };
                    let style = player_style(*owner);
                    let l = if let Some(p) = st
                        .s
                        .fgs
                        .iter()
                        .enumerate()
                        .find(|(p, fg)| fg.is_flagged(pos) && Player(*p as u32) != st.s.controlled)
                        .map(|(p, _)| Player(p as u32))
                    {
                        style::PrintStyledContent(StyledContent::new(player_style(p), "x"))
                    } else {
                        style::PrintStyledContent(StyledContent::new(style, &symbol[0..1]))
                    };
                    let m = style::PrintStyledContent(StyledContent::new(style, &symbol[1..2]));
                    let r = if st.s.fgs[owner.0 as usize].is_flagged(pos) {
                        style::PrintStyledContent(StyledContent::new(player_style(*owner), "P"))
                    } else {
                        style::PrintStyledContent(StyledContent::new(style, &symbol[2..3]))
                    };

                    queue!(st.out, l, m, r)?;
                }
            }
        }

        queue!(st.out, style::Print("\n\r"))?;
    }
    Ok(())
}
