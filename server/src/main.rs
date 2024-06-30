use std::{
    cell::{Cell, RefCell, UnsafeCell},
    fmt::Debug,
    net::SocketAddr,
    time::{Duration, SystemTime},
};

use async_executor::LocalExecutor;
use curseofrust::{
    state::{MultiplayerOpts, State},
    Player, Speed,
};
use curseofrust_cli_parser::Options;
use curseofrust_msg::{bytemuck, C2SData, S2CData, C2S_SIZE, S2C_SIZE};
use curseofrust_net_foundation::{Connection, Handle, Protocol};

const DURATION: Duration = Duration::from_millis(10);

#[derive(Debug)]
struct Client<'sock> {
    id: u32,
    addr: SocketAddr,
    pl: Player,
    socket: UnsafeCell<Connection<'sock>>,
    reads: Cell<usize>,
}

fn main() -> Result<(), DirectBoxedError> {
    fastrand::seed(
        SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or_default()
            .as_secs(),
    );

    let Options {
        basic: mut b_opt,
        multiplayer: m_opt,
        exit,
        protocol,
        ..
    } = curseofrust_cli_parser::parse_to_options(std::env::args_os())?;
    if exit {
        return Ok(());
    }

    let MultiplayerOpts::Server { port } = m_opt else {
        return Err(DirectBoxedError {
            inner: "server information is required".into(),
        });
    };

    let addr: SocketAddr = (
        local_ip_address::local_ip().or_else(|_| local_ip_address::local_ipv6())?,
        port,
    )
        .into();

    let protocol = match protocol {
        curseofrust_cli_parser::Protocol::Tcp => Protocol::Tcp,
        curseofrust_cli_parser::Protocol::Udp => Protocol::Udp,
        #[cfg(feature = "ws")]
        curseofrust_cli_parser::Protocol::WebSocket => Protocol::WebSocket,
        _ => {
            return Err(DirectBoxedError {
                inner: "given protocol is not supported in this build".into(),
            })
        }
    };

    let handle = Handle::bind(addr, protocol)?;
    let listener = handle.listen()?;

    let mut cl: Vec<Client<'_>> = vec![];

    let mut c2s_buf = [0u8; C2S_SIZE];

    println!("[LOBBY] server listening on socket {}", addr);

    futures_lite::future::block_on(async {
        'lobby: loop {
            let Ok((mut connection, peer)) = listener.accept().await else {
                continue;
            };
            if let Ok(nread) = connection.recv(&mut c2s_buf).await {
                if nread >= 1 && c2s_buf[0] > 0 {
                    if !cl.iter().any(|rec| rec.addr == peer) {
                        let id = cl.len() as u32;
                        cl.push(Client {
                            addr: peer,
                            pl: Player(id + 1),
                            id,
                            socket: UnsafeCell::new(connection),
                            reads: Cell::new(0),
                        });

                        println!("[LOBBY] client{}@{} connected", id, peer);
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
    });

    let st = RefCell::new(State::new(b_opt)?);
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

                    for client in &cl {
                        let mut data = data;
                        data.set_player(client.pl);
                        let mut buf = [0u8; S2C_SIZE];
                        let (msg, od) = buf
                            .split_first_mut()
                            .expect("the buffer should longer than one byte");
                        *msg = curseofrust_msg::server_msg::STATE;
                        od.copy_from_slice(bytemuck::bytes_of(&data));
                        let socket = &client.socket;
                        executor
                            .spawn(async move {
                                let ptr = socket.get();
                                let result = unsafe { (*ptr).send(&buf).await };
                                if let Err(e) = result {
                                    eprintln!(
                                        "[PLAY] error sending packet to client{}@{}: {}",
                                        client.id, client.addr, e
                                    );
                                }
                            })
                            .detach()
                    }
                }
            }

            for client in cl.iter() {
                let reads = client.reads.get();
                if reads < 2 {
                    client.reads.set(reads + 1);
                    executor.spawn(recv_fut(client, &st)).detach();
                }
            }
            timer.await;
        }
    }));

    Ok(())
}

async fn recv_fut(cl: &Client<'_>, st: &RefCell<State>) {
    let mut buf = [0u8; C2S_SIZE];
    let sptr = cl.socket.get();
    match unsafe { (*sptr).recv(&mut buf).await } {
        Ok(C2S_SIZE) => {
            let (&msg, od) = buf
                .split_first()
                .expect("the buffer should longer than one byte");
            let data: C2SData = *bytemuck::from_bytes(od);
            let mut st = st.borrow_mut();
            if let Err(e) = curseofrust_msg::apply_c2s_msg(&mut st, cl.pl, msg, data) {
                eprintln!("[PLAY] error performing action for player{}: {}", cl.id, e)
            }
        }
        Ok(nread) => eprintln!(
            "[PLAY] error recv packet from client{}, expected {} bytes, have {}",
            cl.id, C2S_SIZE, nread
        ),
        Err(_) => {}
    }
    cl.reads.set(cl.reads.get() - 1);
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
