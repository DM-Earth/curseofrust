use std::{
    cell::RefCell,
    io::Write,
    net::{SocketAddr, UdpSocket},
    ops::ControlFlow,
};

use async_io::Async;
use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent},
    execute, queue, terminal,
};
use curseofrust::{grid::Tile, Speed};
use curseofrust_msg::{bytemuck, C2SData, S2CData, C2S_SIZE, S2C_SIZE};
use futures_lite::StreamExt as _;
use local_ip_address::{local_ip, local_ipv6};

use crate::{DirectBoxedError, State};

pub(crate) fn run<W: Write>(
    mut st: &mut State<W>,
    server: SocketAddr,
    port: u16,
) -> Result<(), DirectBoxedError> {
    let local: SocketAddr = (
        match server {
            SocketAddr::V4(_) => local_ip(),
            SocketAddr::V6(_) => local_ipv6(),
        }?,
        port,
    )
        .into();

    let socket = UdpSocket::bind(local)?;
    socket.connect(server)?;
    socket.set_nonblocking(true)?;
    let socket = Async::new(socket)?;

    let executor = async_executor::LocalExecutor::new();
    let mut time = 0i32;
    st.s.time = 0;

    let mut s2c_buf = [0u8; S2C_SIZE];

    let mut init = false;

    {
        let st = RefCell::new(&mut st);
        let mut events = crossterm::event::EventStream::new();

        futures_lite::future::block_on(executor.run(async {
            'game: loop {
                let timer = async_io::Timer::after(crate::DURATION);

                if time >= 1600 {
                    time = 0;
                }

                if time % 50 == 0 {
                    const ALIVE_PACKET: [u8; C2S_SIZE] =
                        [curseofrust_msg::client_msg::IS_ALIVE, 0, 0, 0];
                    executor.spawn(socket.send(&ALIVE_PACKET)).detach();
                    if !init {
                        println!("pinging socket {} using {}", server, local)
                    }
                }

                time += 1;

                let fetch_st = async {
                    let nread = socket.recv(&mut s2c_buf).await?;
                    if nread < S2C_SIZE {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            format!("short read: {} < {}", nread, S2C_SIZE),
                        )
                        .into());
                    }

                    let (&msg, data) = s2c_buf
                        .split_first()
                        .expect("the buffer should longer than one byte");
                    let data: S2CData = *bytemuck::from_bytes(data);
                    if msg == curseofrust_msg::server_msg::STATE {
                        let mut st_guard = st.borrow_mut();
                        let st = &mut ***st_guard;
                        curseofrust_msg::apply_s2c_msg(&mut st.s, data)?;
                        crate::output::draw_grid(st)?;
                        Ok(true)
                    } else {
                        Result::<bool, DirectBoxedError>::Ok(false)
                    }
                };

                let recv_input = async {
                    loop {
                        if let Ok(Some(event)) = events.try_next().await {
                            let mut st_guard = st.borrow_mut();
                            let st = &mut ***st_guard;
                            match event {
                                crossterm::event::Event::Key(KeyEvent {
                                    code,
                                    modifiers: _,
                                    kind,
                                    state: _,
                                }) => {
                                    let cursor = st.ui.cursor;
                                    if !matches!(kind, crossterm::event::KeyEventKind::Release) {
                                        macro_rules! msg_send {
                                            ($msg:ident, $info:expr) => {{
                                                let data: C2SData = (st.ui.cursor, $info).into();
                                                let mut buf = [0u8; C2S_SIZE];
                                                let (msg, d) = buf.split_first_mut().expect(
                                                    "the buffer should longer than one byte",
                                                );
                                                *msg = curseofrust_msg::client_msg::$msg;
                                                d.copy_from_slice(bytemuck::bytes_of(&data));
                                                let socket = &socket;
                                                executor
                                                    .spawn(async move {
                                                        let _ = socket.send(&buf).await;
                                                    })
                                                    .detach();
                                            }};
                                            ($msg:ident) => {
                                                msg_send!($msg, 0)
                                            };
                                        }

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
                                                return Result::<
                                                        ControlFlow<()>,
                                                        DirectBoxedError,
                                                    >::Ok(
                                                        ControlFlow::Break(())
                                                    );
                                            }

                                            KeyCode::Char(' ') => {
                                                if st
                                                    .s
                                                    .grid
                                                    .tile(st.ui.cursor)
                                                    .is_some_and(|t| t.is_habitable())
                                                {
                                                    let fg = &st.s.fgs[st.s.controlled.0 as usize];
                                                    if fg.is_flagged(st.ui.cursor) {
                                                        msg_send!(FLAG_OFF);
                                                    } else {
                                                        msg_send!(FLAG_ON);
                                                    }
                                                }
                                            }
                                            KeyCode::Char('x') => msg_send!(FLAG_OFF_ALL),
                                            KeyCode::Char('c') => msg_send!(FLAG_OFF_HALF),
                                            KeyCode::Char('r') | KeyCode::Char('v') => {
                                                msg_send!(BUILD)
                                            }

                                            KeyCode::Char('p') => {
                                                if st.s.speed == Speed::Pause {
                                                    msg_send!(UNPAUSE)
                                                } else {
                                                    msg_send!(PAUSE)
                                                }
                                            }

                                            _ => {}
                                        }
                                    }
                                    if !st.s.grid.tile(st.ui.cursor).is_some_and(Tile::is_visible) {
                                        st.ui.cursor = cursor;
                                    }
                                }
                                crossterm::event::Event::Resize(_, _) => {
                                    queue!(st.out, terminal::Clear(terminal::ClearType::All))?
                                }
                                _ => {}
                            }

                            crate::output::draw_grid(st)?;
                        }
                    }
                };

                if init {
                    let ctl_flow = futures_lite::future::or(recv_input, async {
                        let _ = fetch_st.await;
                        timer.await;
                        Result::<ControlFlow<()>, DirectBoxedError>::Ok(ControlFlow::Continue(()))
                    })
                    .await?;
                    if ctl_flow.is_break() {
                        break 'game;
                    }
                } else {
                    match fetch_st.await {
                        Ok(true) => {
                            let mut st = st.borrow_mut();
                            init = true;
                            execute!(st.out, terminal::EnterAlternateScreen)?;
                            crossterm::terminal::enable_raw_mode()?;
                            execute!(
                                st.out,
                                terminal::Clear(terminal::ClearType::All),
                                cursor::Hide
                            )?;
                        }
                        Ok(_) => {}

                        Err(e) => {
                            eprintln!("error fetching state: {}", e.inner);
                        }
                    }

                    timer.await;
                    // Dead loop if the server is not responding.
                    // User can press `Ctrl-C` to exit.
                }
            }
            Result::<(), DirectBoxedError>::Ok(())
        }))?;
    }

    Ok(())
}
