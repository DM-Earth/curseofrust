use std::{
    fmt::Debug,
    net::{IpAddr, UdpSocket},
    time::SystemTime,
};

use async_io::Async;
use curseofrust::{
    state::{MultiplayerOpts, State},
    Player, Speed,
};
use curseofrust_msg::{ClientRecord, ServerMode, C2S_SIZE};

const DEFAULT_NAME: &str = include_str!("../jim.txt");

fn main() -> Result<(), DirectBoxedError> {
    fastrand::seed(
        SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_secs(),
    );

    let (mut b_opt, m_opt) = curseofrust_cli_parser::parse(std::env::args_os())?;
    let MultiplayerOpts::Server { port } = m_opt else {
        return Err(DirectBoxedError {
            inner: "server information is required".into(),
        });
    };

    let addr = (IpAddr::from([127, 0, 0, 1]), port);
    let socket = UdpSocket::bind(addr)?;
    socket.set_nonblocking(true)?;

    let mut c2s_buf = [0u8; C2S_SIZE];
    let mut cl: Vec<ClientRecord> = vec![];

    'lobby: loop {
        if let Ok((nread, peer_addr)) = socket.recv_from(&mut c2s_buf) {
            if nread >= 1 && c2s_buf[0] > 0 {
                if !cl.iter().any(|rec| rec.addr == peer_addr) {
                    let id = cl.len() as u32;
                    cl.push(ClientRecord {
                        addr: peer_addr,
                        player: Player(id + 1),
                        id,
                        name: DEFAULT_NAME.into(),
                    });

                    println!("[LOBBY] client{}@{} connected", id, peer_addr);
                }

                if cl.len() >= b_opt.clients {
                    b_opt.clients = cl.len();
                    println!(
                        "[LOBBY] server mode switched to PLAY with {} clients",
                        cl.len()
                    );
                    break 'lobby;
                }
            }
        }
    }

    let mut st = State::new(b_opt)?;
    let socket = Async::new(UdpSocket::bind(addr)?)?;
    let mut time = 0i32;
    loop {
        time += 1;
        if time >= 1600 {
            time = 0
        }

        if time.checked_rem(slowdown(st.speed)) == Some(0) && st.speed != Speed::Pause {
            st.kings_move();
            st.simulate();
        }
    }
    todo!()
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
