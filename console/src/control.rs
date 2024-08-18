use std::ops::{ControlFlow, DerefMut};

use crossterm::{
    event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    queue, terminal,
};
use curseofrust::{grid::Tile, Pos};
use curseofrust_cli_parser::ControlMode;
use futures_lite::StreamExt as _;

use crate::{output, DirectBoxedError, State};

pub(crate) trait Client {
    type Error: std::error::Error + Send + Sync + 'static;

    fn quit<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error>;

    fn toggle_flag<W>(&mut self, st: &mut State<W>, pos: Pos) -> Result<(), Self::Error>;
    fn rm_all_flag<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error>;
    fn rm_half_flag<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error>;

    fn build<W>(&mut self, st: &mut State<W>, pos: Pos) -> Result<(), Self::Error>;

    fn faster<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error>;
    fn slower<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error>;
    fn toggle_pause<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error>;
}

pub(crate) async fn accept<W, S>(
    s: impl FnOnce() -> S,
    ct_events: &mut crossterm::event::EventStream,
    mut client: impl Client,
) -> Result<ControlFlow<()>, DirectBoxedError>
where
    W: std::io::Write,
    S: DerefMut<Target = State<W>>,
{
    if let Ok(Some(event)) = ct_events.try_next().await {
        let mut stt = s();
        let st = &mut (*stt);
        let cursor = st.ui.cursor;
        macro_rules! cupd {
            () => {
                if st.ui.cursor == cursor {
                    output::draw_grid(st, Some([cursor, Pos(cursor.0 + 1, cursor.1)]))?;
                } else {
                    output::draw_grid(
                        st,
                        Some([
                            cursor,
                            Pos(cursor.0 + 1, cursor.1),
                            st.ui.cursor,
                            Pos(st.ui.cursor.0 + 1, st.ui.cursor.1),
                        ]),
                    )?;
                }
            };
        }
        macro_rules! pc {
            ($e:expr) => {
                $e.map_err(DirectBoxedError::from)
            };
        }
        match (event, st.control) {
            (
                crossterm::event::Event::Key(KeyEvent {
                    code,
                    modifiers: _,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    state: _,
                }),
                ControlMode::Keyboard | ControlMode::Hybrid,
            ) => {
                let cursor = st.ui.cursor;
                let cursor_x_shift = if st.ui.cursor.1 % 2 == 0 { 0 } else { 1 };
                match code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        st.ui.cursor.1 -= 1;
                        st.ui.cursor.0 += cursor_x_shift;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        st.ui.cursor.1 += 1;
                        st.ui.cursor.0 += cursor_x_shift - 1;
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        st.ui.cursor.0 -= 1;
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        st.ui.cursor.0 += 1;
                    }

                    KeyCode::Char('q') => {
                        pc!(client.quit(st))?;
                        return Ok(ControlFlow::Break(()));
                    }

                    KeyCode::Char(' ') => pc!(client.toggle_flag(st, cursor))?,
                    KeyCode::Char('x') => {
                        pc!(client.rm_all_flag(st))?;
                        output::draw_all_grid(st)?;
                    }
                    KeyCode::Char('c') => {
                        pc!(client.rm_half_flag(st))?;
                        output::draw_all_grid(st)?;
                    }
                    KeyCode::Char('r') | KeyCode::Char('v') => {
                        pc!(client.build(st, cursor))?;
                    }

                    KeyCode::Char('f') => pc!(client.faster(st))?,
                    KeyCode::Char('s') => pc!(client.slower(st))?,
                    KeyCode::Char('p') => pc!(client.toggle_pause(st))?,

                    _ => {}
                }
                if !st.s.grid.tile(st.ui.cursor).is_some_and(Tile::is_visible) {
                    st.ui.cursor = cursor;
                }

                cupd!()
            }
            (
                crossterm::event::Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    state: _,
                }),
                ControlMode::Termux,
            ) => match (code, modifiers) {
                (KeyCode::Esc, _) => {
                    pc!(client.quit(st))?;
                    return Ok(ControlFlow::Break(()));
                }

                (KeyCode::PageUp, _) => pc!(client.faster(st))?,
                (KeyCode::PageDown, _) => pc!(client.slower(st))?,
                (KeyCode::End, _) => pc!(client.toggle_pause(st))?,

                (KeyCode::Home | KeyCode::Up, _) => pc!(client.build(st, cursor))?,

                (KeyCode::Down, KeyModifiers::NONE) => pc!(client.rm_all_flag(st))?,
                (KeyCode::Down, KeyModifiers::ALT) => pc!(client.rm_half_flag(st))?,

                _ => {}
            },
            (
                crossterm::event::Event::Mouse(MouseEvent {
                    kind,
                    column,
                    row,
                    modifiers,
                }),
                ControlMode::Termux | ControlMode::Hybrid,
            ) => {
                let pos = output::rev_pos(column, row, &st.ui, &st.s.grid);
                if let (MouseEventKind::Down(MouseButton::Left), Some(pos), _, _) =
                    (kind, pos, st.control, modifiers)
                {
                    if pos == cursor {
                        pc!(client.toggle_flag(st, cursor))?;
                    } else {
                        st.ui.adjust_cursor(&st.s, pos);
                    }
                }
                cupd!()
            }
            (crossterm::event::Event::Resize(_, _), _) => {
                queue!(st.out, terminal::Clear(terminal::ClearType::All))?;
                output::draw_all_grid(st)?;
            }
            _ => (),
        }
    }
    Ok(ControlFlow::Continue(()))
}
