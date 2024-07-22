use std::{
    convert::Infallible,
    fmt::Debug,
    io::Write,
    ops::ControlFlow,
    time::{Duration, SystemTime},
};

use crossterm::{cursor, execute, terminal};
use curseofrust::{Pos, Speed, FLAG_POWER};
use curseofrust_cli_parser::{ControlMode, Options};

mod client;
mod control;
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
        control_mode,
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
        control: control_mode,
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
    control: ControlMode,
    out: W,
}

struct SingleplayerClient;

impl control::Client for SingleplayerClient {
    type Error = Infallible;

    #[inline(always)]
    fn quit<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn toggle_flag<W>(&mut self, st: &mut State<W>, pos: Pos) -> Result<(), Self::Error> {
        if st.s.grid.tile(pos).is_some_and(|t| t.is_habitable()) {
            let fg = &mut st.s.fgs[st.s.controlled.0 as usize];
            if fg.is_flagged(st.ui.cursor) {
                fg.remove(&st.s.grid, st.ui.cursor, FLAG_POWER);
            } else {
                fg.add(&st.s.grid, st.ui.cursor, FLAG_POWER);
            }
        }
        Ok(())
    }

    #[inline]
    fn rm_all_flag<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error> {
        st.s.fgs[st.s.controlled.0 as usize].remove_with_prob(&st.s.grid, 1.0);
        Ok(())
    }

    #[inline]
    fn rm_half_flag<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error> {
        st.s.fgs[st.s.controlled.0 as usize].remove_with_prob(&st.s.grid, 0.5);
        Ok(())
    }

    #[inline]
    fn build<W>(&mut self, st: &mut State<W>, pos: Pos) -> Result<(), Self::Error> {
        let _ =
            st.s.grid
                .build(&mut st.s.countries[st.s.controlled.0 as usize], pos);
        Ok(())
    }

    #[inline]
    fn faster<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error> {
        st.s.speed = st.s.speed.faster();
        Ok(())
    }

    #[inline]
    fn slower<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error> {
        st.s.speed = st.s.speed.slower();
        Ok(())
    }

    #[inline]
    fn toggle_pause<W>(&mut self, st: &mut State<W>) -> Result<(), Self::Error> {
        if st.s.speed == Speed::Pause {
            st.s.speed = st.s.prev_speed;
        } else {
            st.s.prev_speed = st.s.speed;
            st.s.speed = Speed::Pause
        }
        Ok(())
    }
}

fn run<W: Write>(st: &mut State<W>) -> Result<(), DirectBoxedError> {
    execute!(st.out, terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;
    execute!(
        st.out,
        terminal::Clear(terminal::ClearType::All),
        cursor::Hide
    )?;

    if matches!(st.control, ControlMode::Termux | ControlMode::Hybrid) {
        execute!(st.out, crossterm::event::EnableMouseCapture)?;
    }

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
            control::accept(|| &mut *st, &mut events, SingleplayerClient),
            async {
                timer.await;
                Result::<ControlFlow<(), ()>, DirectBoxedError>::Ok(ControlFlow::Continue(()))
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
