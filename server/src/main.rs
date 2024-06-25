use std::{
    cell::RefCell,
    fmt::Debug,
    marker::PhantomData,
    net::{IpAddr, SocketAddr, UdpSocket},
    time::{Duration, SystemTime},
};

use async_executor::LocalExecutor;
use async_io::Async;
use curseofrust::{
    state::{MultiplayerOpts, State},
    Player, Speed,
};
use curseofrust_msg::{bytemuck, C2SData, ClientRecord, S2CData, C2S_SIZE, S2C_SIZE};

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

struct SocketAddrs<T>(T);

impl<T> std::net::ToSocketAddrs for SocketAddrs<T>
where
    T: IntoIterator<Item = SocketAddr> + Clone,
{
    type Iter = <T as IntoIterator>::IntoIter;

    #[inline(always)]
    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        Ok(self.0.clone().into_iter())
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
    let mut cl: Vec<ClientRecord> = vec![];

    let socket = UdpSocket::bind(addr)?;
    socket.set_nonblocking(true)?;
    let mut c2s_buf = [0u8; C2S_SIZE];

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
    socket.connect(SocketAddrs(cl.iter().map(|client| client.addr)))?;
    let socket = Async::new_nonblocking(socket)?;
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
                            let (msg, od) = buf
                                .split_first_mut()
                                .expect("the buffer should longer than one byte");
                            *msg = curseofrust_msg::server_msg::STATE;
                            od.copy_from_slice(bytemuck::bytes_of(&data));
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

            let recv_fut = || async {
                let mut buf = [0u8; C2S_SIZE];
                match socket.recv_from(&mut buf).await {
                    Ok((C2S_SIZE, peer)) => {
                        let Some(client) = cl.iter().find(|client| client.addr == peer) else {
                            return;
                        };
                        let (&msg, od) = buf
                            .split_first()
                            .expect("the buffer should longer than one byte");
                        let data: C2SData = *bytemuck::from_bytes(od);
                        let mut st = st.borrow_mut();
                        if let Err(e) =
                            curseofrust_msg::apply_c2s_msg(&mut st, client.player, msg, data)
                        {
                            eprintln!("[PLAY] error perform action for player{}: {}", client.id, e)
                        }
                    }
                    Ok((nread, peer)) => eprintln!(
                        "[PLAY] error recv packet from {}, expected {} bytes, have {}",
                        peer, C2S_SIZE, nread
                    ),
                    Err(e) => eprintln!("[PLAY] error recv packet: {}", e),
                }
            };

            futures_lite::future::race(timer, async {
                let mut c = 0usize;
                loop {
                    if socket.readable().await.is_ok() && c <= cl.len() {
                        executor.spawn(recv_fut()).detach();
                        c += 1;
                    }
                }
            })
            .await;
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
