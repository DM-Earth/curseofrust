use std::{
    fmt::Debug,
    io::Write,
    ops::ControlFlow,
    time::{Duration, SystemTime},
};

use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    execute, queue, terminal,
};
use curseofrust::{grid::Tile, Pos, Speed, FLAG_POWER};
use curseofrust_cli_parser::Options;
use futures_lite::StreamExt;

mod client;
mod output;

const DURATION: Duration = Duration::from_millis(10);

fn main() -> Result<(), DirectBoxedError> {
    fastrand::seed(
        SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_secs(),
    );

    let Options {
        basic: b_opt,
        multiplayer: m_opt,
        exit,
        protocol,
        ..
    } = curseofrust_cli_parser::parse_to_options(std::env::args_os())?;
    if exit {
        return Ok(());
    }

    let state = curseofrust::state::State::new(b_opt)?;
    let stdout = std::io::stdout();
    let mut st = State {
        ui: curseofrust::state::UI::new(&state),
        s: state,
        out: stdout,
    };

    match m_opt {
        curseofrust::state::MultiplayerOpts::Server { .. } => Err(DirectBoxedError {
            inner: <Box<dyn std::error::Error>>::from("use dedicated server"),
        }),
        #[cfg(feature = "multiplayer")]
        curseofrust::state::MultiplayerOpts::Client { server, port } => {
            let res = client::run(&mut st, server, port, protocol);
            execute!(st.out, terminal::Clear(terminal::ClearType::All))?;
            terminal::disable_raw_mode()?;
            execute!(st.out, terminal::LeaveAlternateScreen, cursor::Show)?;
            res
        }
        #[cfg(not(feature = "multiplayer"))]
        curseofrust::state::MultiplayerOpts::Client { .. } => Err(DirectBoxedError {
            inner: <Box<dyn std::error::Error>>::from("client feature not enabled"),
        }),

        curseofrust::state::MultiplayerOpts::None => run(&mut st),
    }
}

struct DirectBoxedError {
    inner: BoxedError,
}

impl Debug for DirectBoxedError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T> From<T> for DirectBoxedError
where
    T: std::error::Error + 'static,
{
    #[inline]
    fn from(value: T) -> Self {
        Self {
            inner: Box::new(value),
        }
    }
}

type BoxedError = Box<dyn std::error::Error>;

struct State<W> {
    s: curseofrust::state::State,
    ui: curseofrust::state::UI,
    out: W,
}

fn run<W: Write>(st: &mut State<W>) -> Result<(), DirectBoxedError> {
    execute!(st.out, terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;
    execute!(
        st.out,
        terminal::Clear(terminal::ClearType::All),
        crossterm::event::EnableMouseCapture,
        cursor::Hide
    )?;

    let mut time = 0i32;
    let mut events = crossterm::event::EventStream::new();
    loop {
        let timer = async_io::Timer::after(DURATION);
        time += 1;
        if time >= 1600 {
            time = 0
        }

        if time.checked_rem(slowdown(st.s.speed)) == Some(0) {
            st.s.kings_move();
            st.s.simulate();
            if st.s.show_timeline && st.s.time % 10 == 0 {
                st.s.update_timeline();
            }

            output::draw_all_grid(st)?;
        }

        st.out.flush()?;

        let cond = futures_lite::future::block_on(futures_lite::future::or(
            async {
                loop {
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
                    if let Ok(Some(event)) = events.try_next().await {
                        match event {
                            crossterm::event::Event::Key(KeyEvent {
                                code,
                                modifiers: _,
                                kind,
                                state: _,
                            }) => {
                                let cursor = st.ui.cursor;
                                if !matches!(kind, crossterm::event::KeyEventKind::Release) {
                                    let cursor_x_shift =
                                        if st.ui.cursor.1 % 2 == 0 { 0 } else { 1 };
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
                                            return Ok(ControlFlow::Break(()));
                                        }

                                        KeyCode::Char(' ') => {
                                            if st
                                                .s
                                                .grid
                                                .tile(st.ui.cursor)
                                                .is_some_and(|t| t.is_habitable())
                                            {
                                                let fg = &mut st.s.fgs[st.s.controlled.0 as usize];
                                                if fg.is_flagged(st.ui.cursor) {
                                                    fg.remove(&st.s.grid, st.ui.cursor, FLAG_POWER);
                                                } else {
                                                    fg.add(&st.s.grid, st.ui.cursor, FLAG_POWER);
                                                }
                                            }
                                        }
                                        KeyCode::Char('x') => {
                                            st.s.fgs[st.s.controlled.0 as usize]
                                                .remove_with_prob(&st.s.grid, 1.0);
                                            output::draw_all_grid(st)?;
                                        }
                                        KeyCode::Char('c') => {
                                            st.s.fgs[st.s.controlled.0 as usize]
                                                .remove_with_prob(&st.s.grid, 0.5);
                                            output::draw_all_grid(st)?;
                                        }
                                        KeyCode::Char('r') | KeyCode::Char('v') => {
                                            let _ = st.s.grid.build(
                                                &mut st.s.countries[st.s.controlled.0 as usize],
                                                st.ui.cursor,
                                            );
                                        }

                                        KeyCode::Char('f') => st.s.speed = st.s.speed.faster(),
                                        KeyCode::Char('s') => st.s.speed = st.s.speed.slower(),
                                        KeyCode::Char('p') => {
                                            if st.s.speed == Speed::Pause {
                                                st.s.speed = st.s.prev_speed;
                                            } else {
                                                st.s.prev_speed = st.s.speed;
                                                st.s.speed = Speed::Pause
                                            }
                                        }

                                        _ => {}
                                    }
                                }
                                if !st.s.grid.tile(st.ui.cursor).is_some_and(Tile::is_visible) {
                                    st.ui.cursor = cursor;
                                }

                                cupd!()
                            }
                            crossterm::event::Event::Mouse(MouseEvent {
                                kind,
                                column,
                                row,
                                modifiers: _,
                            }) => {
                                // unreachable!("{kind:?}");
                                let pos = output::rev_pos(column, row, &st.ui);
                                match (kind, pos) {
                                    (MouseEventKind::Down(MouseButton::Left), Some(pos)) => {
                                        if pos == cursor {
                                            if st
                                                .s
                                                .grid
                                                .tile(st.ui.cursor)
                                                .is_some_and(|t| t.is_habitable())
                                            {
                                                let fg = &mut st.s.fgs[st.s.controlled.0 as usize];
                                                if fg.is_flagged(st.ui.cursor) {
                                                    fg.remove(&st.s.grid, st.ui.cursor, FLAG_POWER);
                                                } else {
                                                    fg.add(&st.s.grid, st.ui.cursor, FLAG_POWER);
                                                }
                                            }
                                        } else {
                                            st.ui.adjust_cursor(&st.s, pos);
                                        }
                                        cupd!()
                                    }
                                    _ => {}
                                }
                            }
                            crossterm::event::Event::Resize(_, _) => {
                                queue!(st.out, terminal::Clear(terminal::ClearType::All))?;
                                output::draw_all_grid(st)?;
                            }
                            _ => (),
                        }
                    }
                }
            },
            async {
                timer.await;
                Result::<_, std::io::Error>::Ok(ControlFlow::Continue(()))
            },
        ))?;

        if cond.is_break() {
            break;
        }
    }

    terminal::disable_raw_mode()?;
    execute!(st.out, terminal::LeaveAlternateScreen, cursor::Show)?;

    Ok(())
}

#[inline]
fn slowdown(speed: Speed) -> i32 {
    match speed {
        Speed::Pause => 0,
        Speed::Slowest => 160,
        Speed::Slower => 80,
        Speed::Slow => 40,
        Speed::Normal => 20,
        Speed::Fast => 10,
        Speed::Faster => 5,
        Speed::Fastest => 2,
    }
}
