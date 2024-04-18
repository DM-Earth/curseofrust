//! Output functions for the console.

use std::io::Write;

use crossterm::{
    cursor, queue,
    style::{self, Attribute, Color, ContentStyle, StyledContent},
    terminal::{self, ClearType},
};
use curseofrust::{Player, Pos};

use crate::State;

const GRASS: &str = " - ";
const MOUNTAIN: &str = "/\\^";
const MINE: &str = "/$\\";
const VILLAGE: &str = " n ";
const TOWN: &str = "i=i";
const FORTRESS: &str = "W#W";

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
        Player(3) => Color::Yellow,
        Player(4) => Color::Red,
        Player(5) => Color::Magenta,
        Player(6) => Color::Cyan,
        Player(7) => Color::DarkGrey,
        _ => Color::Reset,
    }
}

#[inline]
fn pop_to_symbol(pop: u16) -> &'static str {
    match pop {
        0 => GRASS,
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
        queue!(st.out, cursor::MoveTo(1, y as u16 + 1))?;
        if y % 2 == 0 {
            queue!(st.out, style::Print("  "))?;
        }

        for x in 0..st.s.grid.width() {
            queue!(
                st.out,
                cursor::MoveTo(
                    ((x * 4 + y * 2 + 1) as i16 - st.ui.xskip as i16 * 4).max(0) as u16,
                    y as u16 + 1
                )
            )?;
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
                    queue!(
                        st.out,
                        style::PrintStyledContent(StyledContent::new(
                            ContentStyle {
                                attributes: style::Attribute::Bold.into(),
                                ..Default::default()
                            },
                            l_sym
                        ))
                    )?;
                };
            }
            match tile {
                curseofrust::grid::Tile::Void => {
                    cursor!();
                    queue!(st.out, style::Print("   "))?;
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
                            ContentStyle {
                                foreground_color: Some(Color::Yellow),
                                ..player_style(*owner)
                            },
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
                    let r = if st.s.fgs[st.s.controlled.0 as usize].is_flagged(pos) {
                        style::PrintStyledContent(StyledContent::new(Default::default(), "P"))
                    } else {
                        style::PrintStyledContent(StyledContent::new(style, &symbol[2..3]))
                    };

                    queue!(st.out, l, m, r)?;
                }
            }
        }
    }

    queue!(
        st.out,
        cursor::MoveTo(0, st.s.grid.height() as u16),
        terminal::Clear(ClearType::CurrentLine),
        style::PrintStyledContent(StyledContent::new(
            ContentStyle {
                foreground_color: Some(player_color(st.s.controlled)),
                attributes: Attribute::Reverse.into(),
                ..Default::default()
            },
            format!("  {}  ", st.s.countries[st.s.controlled.0 as usize].gold)
        )),
        style::Print("    ")
    )?;

    if let Some(tile) = st.s.grid.tile(st.ui.cursor) {
        for (p, pop) in tile.units().iter().enumerate() {
            queue!(
                st.out,
                style::Print("  "),
                style::PrintStyledContent(StyledContent::new(player_style(Player(p as u32)), *pop))
            )?;
        }
    }

    Ok(())
}
