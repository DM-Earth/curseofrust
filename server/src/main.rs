use std::{
    cell::RefCell,
    fmt::Debug,
    marker::PhantomData,
    net::{IpAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use async_executor::LocalExecutor;
use async_io::Async;
use curseofrust::{
    state::{MultiplayerOpts, State},
    Player, Speed,
};
use curseofrust_msg::{bytemuck, ClientRecord, S2CData, C2S_SIZE, S2C_SIZE};

const DEFAULT_NAME: &str = include_str!("../jim.txt");
const DURATION: Duration = Duration::from_millis(10);

struct TaskDetacher<T>(PhantomData<T>);

impl<T> Extend<async_executor::Task<T>> for TaskDetacher<T> {
    #[inline(always)]
    fn extend<I: IntoIterator<Item = async_executor::Task<T>>>(&mut self, iter: I) {
        for task in iter {
            task.detach();
        }
    }
}

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

    let st = RefCell::new(State::new(b_opt)?);
    let socket = Async::new(UdpSocket::bind(addr)?)?;
    let mut time = 0i32;
    let executor = LocalExecutor::new();

    futures_lite::future::block_on(executor.run(async {
        loop {
            let timer = async_io::Timer::after(DURATION);
            time += 1;
            if time >= 1600 {
                time = 0
            }

            {
                let mut st = st.borrow_mut();
                if time.checked_rem(slowdown(st.speed)) == Some(0) && st.speed != Speed::Pause {
                    st.kings_move();
                    st.simulate();
                    let data = S2CData::new(Default::default(), &st);

                    executor.spawn_many(
                        cl.iter().map(|client| {
                            let mut data = data;
                            data.set_player(client.player);
                            let mut buf = [0u8; S2C_SIZE];
                            buf[0] = curseofrust_msg::server_msg::STATE;
                            buf[1..].copy_from_slice(bytemuck::bytes_of(&data));
                            let socket = &socket;
                            async move {
                                let result = socket.send_to(&buf, client.addr).await;
                                if let Err(e) = result {
                                    eprintln!(
                                        "[PLAY] error sending UDP packet to client{}@{}: {}",
                                        client.id, client.addr, e
                                    );
                                }
                            }
                        }),
                        &mut TaskDetacher(PhantomData),
                    )
                }
            }

            timer.await;
        }
    }));

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
