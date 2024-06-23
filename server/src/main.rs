use std::{
    fmt::Debug,
    net::{IpAddr, UdpSocket},
    time::SystemTime,
};

use async_io::Async;
use curseofrust::{
    state::{MultiplayerOpts, State},
    Speed,
};
use curseofrust_msg::{ClientRecord, ServerMode, C2S_SIZE};

fn main() -> Result<(), DirectBoxedError> {
    fastrand::seed(
        SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_secs(),
    );

    let (b_opt, m_opt) = curseofrust_cli_parser::parse(std::env::args_os())?;
    let MultiplayerOpts::Server { port } = m_opt else {
        return Err(DirectBoxedError {
            inner: "server information is required".into(),
        });
    };

    let mut st = State::new(b_opt)?;
    let socket = Async::new(UdpSocket::bind((IpAddr::from([127, 0, 0, 1]), port))?)?;
    let mut mode = ServerMode::Lobby;

    let mut c2s_buf = [0u8; C2S_SIZE];

    loop {
        match mode {
            ServerMode::Lobby => futures_lite::future::block_on(async {
                if let Ok((nread, peer_addr)) = socket.recv_from(&mut c2s_buf).await {
                    if nread >= 1 && c2s_buf[0] > 0 {}
                }
            }),
            ServerMode::Play => todo!(),
        }
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
