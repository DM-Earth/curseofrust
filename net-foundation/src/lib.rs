//! Fundamental async socket backend based on `unisock`.

#![warn(missing_docs)]

use std::net::{SocketAddr, ToSocketAddrs};

use unisock::*;

mod util;

#[allow(unused_imports)]
use util::*;

/// The protocol of the socket.
///
/// The default protocol is `Udp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Protocol {
    /// TCP.
    Tcp,
    /// UDP using only **a single socket**.
    #[default]
    Udp,
    /// WebSocket.
    #[cfg(feature = "ws")]
    WebSocket,
}

/// The main handler.
#[derive(Debug)]
pub struct Handle(HandleInner);

#[derive(Debug)]
enum HandleInner {
    Tcp(unisock_smol::Tcp),
    Udp(unisock_smol::UdpSingle),
    #[cfg(feature = "ws")]
    WebSocket(unisock_smol_tungstenite::WebSocket),
}

macro_rules! call {
    (const: $this:expr, $thist:ident => $fun:ident($($i:expr),*$(,)?)) => {
        match $this {
            $thist::Tcp(ref back) => back.$fun($($i),*),
            $thist::Udp(ref back) => back.$fun($($i),*),
            #[cfg(feature = "ws")]
            $thist::WebSocket(ref back) => back.$fun($($i),*).map_err(err_ws2io),
        }
    };
    ($this:expr, $thist:ident => $fun:ident($($i:expr),*$(,)?).await) => {
        match $this {
            $thist::Tcp(ref mut back) => back.$fun($($i),*).await,
            $thist::Udp(ref mut back) => back.$fun($($i),*).await,
            #[cfg(feature = "ws")]
            $thist::WebSocket(ref mut back) => back.$fun($($i),*).await.map_err(err_ws2io),
        }
    };
}

impl Handle {
    /// Connect to the address with the specified protocol.
    pub fn bind<A>(addr: A, protocol: Protocol) -> Result<Self, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        let mut err = None;
        for addr in addr.to_socket_addrs()? {
            match protocol {
                Protocol::Tcp => match unisock_smol::Tcp::bind(addr) {
                    Ok(back) => return Ok(Self(HandleInner::Tcp(back))),
                    Err(e) => err = Some(e),
                },
                Protocol::Udp => match unisock_smol::UdpSingle::bind(addr) {
                    Ok(back) => return Ok(Self(HandleInner::Udp(back))),
                    Err(e) => err = Some(e),
                },
                #[cfg(feature = "ws")]
                Protocol::WebSocket => match unisock_smol_tungstenite::WebSocket::bind(addr) {
                    Ok(back) => return Ok(Self(HandleInner::WebSocket(back))),
                    Err(e) => err = Some(err_ws2io(e)),
                },
            }
        }

        Err(err.unwrap_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "no valid address found")
        }))
    }

    /// Returns the listener.
    pub fn listen(&self) -> Result<Listener, std::io::Error> {
        match &self.0 {
            HandleInner::Tcp(back) => back.listen().map(|l| Listener(ListenerInner::Tcp(l))),
            HandleInner::Udp(back) => Ok(Listener(ListenerInner::Udp(back))),
            #[cfg(feature = "ws")]
            HandleInner::WebSocket(back) => back
                .listen()
                .map(|l| Listener(ListenerInner::WebSocket(l)))
                .map_err(err_ws2io),
        }
    }

    /// Connect to the address.
    pub async fn connect<A>(&self, addr: A) -> Result<Connection, std::io::Error>
    where
        A: ToSocketAddrs,
    {
        let mut err = None;
        for addr in addr.to_socket_addrs()? {
            match &self.0 {
                HandleInner::Tcp(back) => match back.connect(addr).await {
                    Ok(conn) => return Ok(Connection(ConnectionInner::Tcp(conn))),
                    Err(e) => err = Some(e),
                },
                HandleInner::Udp(back) => match back.connect(addr).await {
                    Ok(conn) => return Ok(Connection(ConnectionInner::Udp(conn))),
                    Err(e) => err = Some(e),
                },
                #[cfg(feature = "ws")]
                HandleInner::WebSocket(back) => match back.connect(addr).await {
                    Ok(conn) => return Ok(Connection(ConnectionInner::WebSocket(conn))),
                    Err(e) => err = Some(err_ws2io(e)),
                },
            }
        }

        Err(err.unwrap_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "no valid address found")
        }))
    }
}

/// The listener.
#[derive(Debug)]
pub struct Listener<'a>(ListenerInner<'a>);

#[derive(Debug)]
enum ListenerInner<'a> {
    Tcp(unisock_smol::tcp::Listener),
    Udp(&'a unisock_smol::UdpSingle),
    #[cfg(feature = "ws")]
    WebSocket(unisock_smol_tungstenite::Listener),
}

impl Listener<'_> {
    /// Accept a connection.
    pub async fn accept(&self) -> Result<(Connection, SocketAddr), std::io::Error> {
        match &self.0 {
            ListenerInner::Tcp(back) => back
                .accept()
                .await
                .map(|(c, a)| (Connection(ConnectionInner::Tcp(c)), a)),
            ListenerInner::Udp(back) => back
                .accept()
                .await
                .map(|(c, a)| (Connection(ConnectionInner::Udp(c)), a)),
            #[cfg(feature = "ws")]
            ListenerInner::WebSocket(back) => back
                .accept()
                .await
                .map(|(c, a)| (Connection(ConnectionInner::WebSocket(c)), a))
                .map_err(err_ws2io),
        }
    }
}

/// The connection.
#[derive(Debug)]
pub struct Connection<'a>(ConnectionInner<'a>);

#[derive(Debug)]
enum ConnectionInner<'a> {
    Tcp(unisock_smol::tcp::Connection),
    Udp(unisock_smol::udp_single_sock::Connection<'a>),
    #[cfg(feature = "ws")]
    WebSocket(unisock_smol_tungstenite::Connection),
}

impl Connection<'_> {
    /// Send data.
    pub async fn send(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        call!(self.0, ConnectionInner => write(data).await)
    }

    /// Receive data.
    pub async fn recv(&mut self, data: &mut [u8]) -> Result<usize, std::io::Error> {
        call!(self.0, ConnectionInner => read(data).await)
    }

    /// Poll the connection for readability.
    pub fn poll_readable(&self, cx: &mut std::task::Context<'_>) -> bool {
        call!(const: self.0, ConnectionInner => poll_readable(cx))
    }

    /// Close the connection.
    pub async fn close(self) -> Result<(), std::io::Error> {
        match self.0 {
            ConnectionInner::Tcp(back) => back.close().await,
            ConnectionInner::Udp(back) => back.close().await,
            #[cfg(feature = "ws")]
            ConnectionInner::WebSocket(back) => back.close().await.map_err(err_ws2io),
        }
    }
}
