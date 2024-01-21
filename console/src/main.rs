use std::{
    fmt::Debug,
    io::Write,
    time::{Duration, SystemTime},
};

use crossterm::{cursor, execute, queue, terminal};
use curseofrust::Speed;

mod output;

fn main() -> Result<(), DirectBoxedError> {
    fastrand::seed(
        SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_secs(),
    );
    let (b_opt, m_opt) = curseofrust_cli_parser::parse(std::env::args_os())?;
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

    run(&mut st)?;

    crossterm::terminal::disable_raw_mode()?;
    execute!(st.out, terminal::LeaveAlternateScreen, cursor::Show)?;

    Ok(())
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
    let mut time = 0;
    loop {
        time += 1;
        if time >= 1600 {
            time = 0
        }

        let mut redraw = false;

        if time % slowdown(st.s.speed) == 0 {
            st.s.kings_move();
            st.s.simulate();
            if st.s.show_timeline && st.s.time % 10 == 0 {
                st.s.update_timeline();
            }

            redraw = true;
        }

        if redraw {
            queue!(st.out, cursor::MoveTo(0, 0))?;
            output::draw_grid(st)?;
        }

        st.out.flush()?;
        std::thread::sleep(Duration::from_millis(10));
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
