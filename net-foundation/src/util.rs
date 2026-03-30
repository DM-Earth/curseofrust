#[cfg(feature = "ws")]
pub(crate) fn err_ws2io(err: unisock_smol_tungstenite::WsError) -> std::io::Error {
    match err {
        unisock_smol_tungstenite::WsError::ConnectionClosed => std::io::Error::new(
            std::io::ErrorKind::ConnectionAborted,
            "(ws) connection closed",
        ),
        unisock_smol_tungstenite::WsError::AlreadyClosed => {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "(ws) already closed")
        }
        unisock_smol_tungstenite::WsError::Io(io) => io,
        unisock_smol_tungstenite::WsError::Tls(err) => {
            std::io::Error::other(format!("(ws) tls error: {}", err))
        }
        unisock_smol_tungstenite::WsError::Capacity(err) => std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("(ws) capacity error: {}", err),
        ),
        unisock_smol_tungstenite::WsError::Protocol(err) => {
            std::io::Error::other(format!("(ws) protocol error: {}", err))
        }
        unisock_smol_tungstenite::WsError::WriteBufferFull(msg) => std::io::Error::other(format!(
            "(ws) write buffer full: the message length is {}",
            msg.len()
        )),
        unisock_smol_tungstenite::WsError::Utf8(msg) => std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("(ws) utf8 error: invalid utf8 string: {msg}"),
        ),
        unisock_smol_tungstenite::WsError::AttackAttempt => {
            std::io::Error::other("(ws) attack attempt: the peer is trying to attack the server")
        }
        unisock_smol_tungstenite::WsError::Url(err) => std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("(ws) url error: {}", err),
        ),
        unisock_smol_tungstenite::WsError::Http(resp) => std::io::Error::other(format!(
            "(ws) http error: {}",
            resp.body()
                .as_deref()
                .map_or(std::borrow::Cow::Borrowed(""), |body| {
                    String::from_utf8_lossy(body)
                }),
        )),
        unisock_smol_tungstenite::WsError::HttpFormat(err) => std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("(ws) http format error: {}", err),
        ),
    }
}
