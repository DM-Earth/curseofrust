#![cfg(feature = "multiplayer")]

use std::{
    cell::{RefCell, UnsafeCell},
    convert::Infallible,
    io::Write,
    net::SocketAddr,
    ops::{ControlFlow, Deref, DerefMut},
};

use async_executor::{Executor, LocalExecutor};
use crossterm::{cursor, execute, terminal};
use curseofrust::Pos;
use curseofrust_cli_parser::ControlMode;
use curseofrust_msg::{bytemuck, client_msg::*, C2SData, S2CData, C2S_SIZE, S2C_SIZE};
use curseofrust_net_foundation::{Connection, Handle, Protocol};
use local_ip_address::{local_ip, local_ipv6};

use crate::{control, DirectBoxedError, EventStream, State};

#[derive(Copy, Clone)]
struct MultiplayerClient<'env> {
    executor: *const Executor<'env>,
    socket: *const UnsafeCell<Connection<'env>>,
}

impl MultiplayerClient<'_> {
    fn send_with_info(&self, cursor: Pos, msg: u8, info: u8) {
        let data: C2SData = (cursor, info).into();
        let mut buf = [0u8; C2S_SIZE];
        let (m, d) = buf
            .split_first_mut()
            .expect("the buffer should longer than one byte");
        *m = msg;
        d.copy_from_slice(bytemuck::bytes_of(&data));
        unsafe {
            let socket = &mut (*UnsafeCell::raw_get(self.socket));
            (*self.executor)
                .spawn(async move {
                    let _ = socket.send(&buf).await;
                })
                .detach();
        }
    }

    #[inline]
    fn send(&self, cursor: Pos, msg: u8) {
        self.send_with_info(cursor, msg, 0u8)
    }
}

impl control::Client for MultiplayerClient<'_> {
    type Error = Infallible;

    #[inline(always)]
    fn quit<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn toggle_flag<W>(&mut self, st: &mut State<W>, pos: Pos) -> Result<(), Self::Error> {
        if st
            .s
            .grid
            .tile(st.ui.cursor)
            .is_some_and(|t| t.is_habitable())
        {
            let fg = &st.s.fgs[st.s.controlled.0 as usize];
            if fg.is_flagged(st.ui.cursor) {
                self.send(pos, FLAG_OFF);
            } else {
                self.send(pos, FLAG_ON);
            }
        }
        Ok(())
    }

    #[inline]
    fn rm_all_flag<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        self.send(Pos::default(), FLAG_OFF_ALL);
        Ok(())
    }

    #[inline]
    fn rm_half_flag<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        self.send(Pos::default(), FLAG_OFF_HALF);
        Ok(())
    }

    #[inline]
    fn build<W>(&mut self, _st: &mut State<W>, pos: Pos) -> Result<(), Self::Error> {
        self.send(pos, BUILD);
        Ok(())
    }

    #[inline(always)]
    fn faster<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline(always)]
    fn slower<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn toggle_pause<W>(&mut self, _st: &mut State<W>) -> Result<(), Self::Error> {
        self.send(Pos::default(), PAUSE);
        Ok(())
    }
}

pub(crate) fn run<W: Write>(
    st: &mut State<W>,
    server: SocketAddr,
    port: u16,
    protocol: curseofrust_cli_parser::Protocol,
) -> Result<(), DirectBoxedError> {
    let local: SocketAddr = (
        match server {
            SocketAddr::V4(_) => local_ip(),
            SocketAddr::V6(_) => local_ipv6(),
        }?,
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

    let handle = Handle::bind(local, protocol)?;
    let socket = UnsafeCell::new(futures_lite::future::block_on(handle.connect(server))?);

    let executor = async_executor::Executor::new();
    let mut time = 0i32;
    st.s.time = 0;

    let mut s2c_buf = [0u8; S2C_SIZE];

    let mut init = false;

    {
        #[repr(transparent)]
        struct WrappingCell<'a, T>(std::cell::RefMut<'a, T>);

        impl<'a, T> Deref for WrappingCell<'a, &'a mut T> {
            type Target = T;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<'a, T> DerefMut for WrappingCell<'a, &'a mut T> {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        let st = RefCell::new(&mut *st);

        futures_lite::future::block_on(executor.run(async {
            'game: loop {
                let timer = async_io::Timer::after(crate::DURATION);

                if time >= 1600 {
                    time = 0;
                }

                if time % 50 == 0 {
                    const ALIVE_PACKET: [u8; C2S_SIZE] =
                        [curseofrust_msg::client_msg::IS_ALIVE, 0, 0, 0];

                    unsafe {
                        executor.spawn((*socket.get()).send(&ALIVE_PACKET)).detach();
                    }
                    if !init {
                        println!("pinging socket {} using {}", server, local)
                    }
                }

                time += 1;

                let fetch_st = async {
                    let nread = unsafe { (*socket.get()).recv(&mut s2c_buf).await? };
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
                        let st = &mut **st_guard;
                        curseofrust_msg::apply_s2c_msg(&mut st.s, data)?;
                        crate::output::draw_all_grid(st)?;
                        Ok(true)
                    } else {
                        Result::<bool, DirectBoxedError>::Ok(false)
                    }
                };

                let client = MultiplayerClient {
                    executor: &executor,
                    socket: &socket,
                };

                let recv_input = async {
                    loop {
                        if let Ok(ControlFlow::Break(_)) =
                            control::accept(|| WrappingCell(st.borrow_mut()), EventStream, client)
                                .await
                        {
                            return Ok(ControlFlow::Break(()));
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
                            crossterm::terminal::enable_raw_mode()?;
                            execute!(st.out, terminal::EnterAlternateScreen)?;
                            if matches!(st.control, ControlMode::Termux | ControlMode::Hybrid) {
                                execute!(st.out, crossterm::event::EnableMouseCapture)?;
                            }
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
