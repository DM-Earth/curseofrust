use std::{
    fmt::Debug,
    io::Write,
    time::{Duration, SystemTime},
};

use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent},
    execute, queue, terminal,
};
use curseofrust::{Speed, FLAG_POWER};
use futures_lite::StreamExt;

mod output;

fn main() -> Result<(), DirectBoxedError> {
    fastrand::seed(
        SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_secs(),
    );
    let (b_opt, _m_opt) = curseofrust_cli_parser::parse(std::env::args_os())?;
    let state = curseofrust::state::State::new(b_opt)?;
    let stdout = std::io::stdout();
    let mut st = State {
        ui: curseofrust::state::UI::new(&state),
        s: state,
        out: stdout,
    };

    execute!(st.out, terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;
    execute!(
        st.out,
        terminal::Clear(terminal::ClearType::All),
        cursor::Hide
    )?;

    let result = run(&mut st);

    crossterm::terminal::disable_raw_mode()?;
    execute!(st.out, terminal::LeaveAlternateScreen, cursor::Show)?;

    result
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
    T: std::error::Error + Send + Sync + 'static,
{
    #[inline]
    fn from(value: T) -> Self {
        Self {
            inner: Box::new(value),
        }
    }
}

type BoxedError = Box<dyn std::error::Error + Send + Sync>;

struct State<W> {
    s: curseofrust::state::State,
    ui: curseofrust::state::UI,
    out: W,
}

fn run<W: Write>(st: &mut State<W>) -> Result<(), DirectBoxedError> {
    const DURATION: Duration = Duration::from_millis(10);

    let mut time = 0;
    let mut events = crossterm::event::EventStream::new();
    loop {
        time += 1;
        if time >= 1600 {
            time = 0
        }

        if time % slowdown(st.s.speed) == 0 {
            st.s.kings_move();
            st.s.simulate();
            if st.s.show_timeline && st.s.time % 10 == 0 {
                st.s.update_timeline();
            }

            output::draw_grid(st)?;
        }

        st.out.flush()?;

        let (cond, _) = futures_lite::future::block_on(futures_lite::future::zip(
            async {
                let cond = futures_lite::future::or(events.try_next(), async {
                    async_io::Timer::after(DURATION).await;
                    Ok(None)
                })
                .await;
                if let Ok(Some(event)) = cond {
                    match event {
                        crossterm::event::Event::Key(KeyEvent {
                            code,
                            modifiers: _,
                            kind,
                            state: _,
                        }) => {
                            let cursor = st.ui.cursor;
                            if !matches!(kind, crossterm::event::KeyEventKind::Release) {
                                match code {
                                    KeyCode::Up | KeyCode::Char('k') => {
                                        st.ui.cursor.1 -= 1;
                                    }
                                    KeyCode::Down | KeyCode::Char('j') => {
                                        st.ui.cursor.1 += 1;
                                    }
                                    KeyCode::Left | KeyCode::Char('h') => {
                                        st.ui.cursor.0 -= 1;
                                    }
                                    KeyCode::Right | KeyCode::Char('l') => {
                                        st.ui.cursor.0 += 1;
                                    }

                                    KeyCode::Char('q') => {
                                        return Ok(true);
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
                                    KeyCode::Char('x') => st.s.fgs[st.s.controlled.0 as usize]
                                        .remove_with_prob(&st.s.grid, 1.0),
                                    KeyCode::Char('c') => st.s.fgs[st.s.controlled.0 as usize]
                                        .remove_with_prob(&st.s.grid, 0.5),
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

                                    _ => (),
                                }
                            }
                            if !st.s.grid.tile(st.ui.cursor).is_some_and(|t| t.is_visible()) {
                                st.ui.cursor = cursor;
                            }
                        }
                        crossterm::event::Event::Resize(_, _) => {
                            queue!(st.out, terminal::Clear(terminal::ClearType::All))?
                        }
                        _ => (),
                    }

                    output::draw_grid(st)?;
                }
                Result::<_, std::io::Error>::Ok(false)
            },
            async_io::Timer::after(DURATION),
        ));

        if cond? {
            break;
        }
    }
    Ok(())
}

#[inline]
fn slowdown(speed: Speed) -> i32 {
    match speed {
        Speed::Pause => i32::MAX,
        Speed::Slowest => 160,
        Speed::Slower => 80,
        Speed::Slow => 40,
        Speed::Normal => 20,
        Speed::Fast => 10,
        Speed::Faster => 5,
        Speed::Fastest => 2,
    }
}
