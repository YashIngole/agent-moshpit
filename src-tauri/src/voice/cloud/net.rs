//! A WebSocket to a provider over TLS, verified against the system's own certificates (so a
//! company's root works as it does in the browser). Blocking, with short reads, so one thread
//! can send audio and read results in turn.
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;

/// A socket whose read timeout reads as "nothing yet", which both TLS and the WebSocket keep
/// going after. Windows reports a timeout differently from the others; this is the same on all.
pub struct Patient(TcpStream);

impl Read for Patient {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf).map_err(|e| if e.kind() == std::io::ErrorKind::TimedOut { std::io::ErrorKind::WouldBlock.into() } else { e })
    }
}
impl Write for Patient {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

pub type Socket = tungstenite::WebSocket<Stream>;

pub enum Stream {
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, Patient>>),
    /// Plain WebSocket, only to a test server on this computer (debug builds).
    Plain(Patient),
}
impl Stream {
    /// The socket under it. Its timeouts are set here: a clone of it is another handle, and on
    /// Windows a timeout set on one handle does not reach reads made through the other.
    fn tcp(&self) -> &TcpStream {
        match self {
            Stream::Tls(tls) => &tls.get_ref().0,
            Stream::Plain(plain) => &plain.0,
        }
    }
}
impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Tls(tls) => tls.read(buf),
            Stream::Plain(plain) => plain.read(buf),
        }
    }
}
impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Tls(tls) => tls.write(buf),
            Stream::Plain(plain) => plain.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Tls(tls) => tls.flush(),
            Stream::Plain(plain) => plain.flush(),
        }
    }
}

/// Why a connection could not be made, in terms the provider's name is put into.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// The provider answered the handshake with this HTTP status, and perhaps a reason.
    Status(u16, String),
    Unreachable(String),
    Certificate(String),
}

/// Opens the socket: DNS, TCP, TLS and the WebSocket handshake, each bounded in time.
pub fn connect(url: &str, headers: &[(&'static str, String)]) -> Result<Socket, Failure> {
    let parsed = url::Url::parse(url).map_err(|e| Failure::Unreachable(e.to_string()))?;
    let host = parsed.host_str().ok_or_else(|| Failure::Unreachable("no host".into()))?.to_string();
    let secure = match parsed.scheme() {
        "wss" => true,
        // A plain socket only reaches this computer, and only a debug build makes one.
        "ws" if cfg!(debug_assertions) && (host == "127.0.0.1" || host == "localhost") => false,
        other => return Err(Failure::Unreachable(format!("unsupported scheme {other}"))),
    };
    let port = parsed.port_or_known_default().unwrap_or(443);
    let addrs: Vec<_> = (host.as_str(), port).to_socket_addrs().map_err(|e| Failure::Unreachable(e.to_string()))?.collect();
    let mut tcp = None;
    let mut last = String::from("no address");
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(stream) => {
                tcp = Some(stream);
                break;
            }
            Err(e) => last = e.to_string(),
        }
    }
    let tcp = tcp.ok_or(Failure::Unreachable(last))?;
    let _ = tcp.set_nodelay(true);
    tcp.set_read_timeout(Some(Duration::from_secs(8))).map_err(|e| Failure::Unreachable(e.to_string()))?;
    tcp.set_write_timeout(Some(Duration::from_secs(8))).map_err(|e| Failure::Unreachable(e.to_string()))?;
    let stream = if secure {
        let _ = rustls::crypto::ring::default_provider().install_default();
        use rustls_platform_verifier::ConfigVerifierExt;
        let config = rustls::ClientConfig::with_platform_verifier().map_err(|e| Failure::Certificate(e.to_string()))?;
        let name = rustls::pki_types::ServerName::try_from(host.clone()).map_err(|e| Failure::Unreachable(e.to_string()))?;
        let connection = rustls::ClientConnection::new(Arc::new(config), name).map_err(|e| Failure::Certificate(e.to_string()))?;
        Stream::Tls(Box::new(rustls::StreamOwned::new(connection, Patient(tcp))))
    } else {
        Stream::Plain(Patient(tcp))
    };
    let mut request = url.into_client_request().map_err(|e| Failure::Unreachable(e.to_string()))?;
    for (name, value) in headers {
        let value = HeaderValue::from_str(value).map_err(|_| Failure::Status(401, "The key has characters a key cannot have.".into()))?;
        request.headers_mut().insert(*name, value);
    }
    let (socket, _) = tungstenite::client::client(request, stream).map_err(|e| match e {
        tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response)) => {
            let reason = response.body().as_ref().map(|body| String::from_utf8_lossy(body).chars().take(300).collect()).unwrap_or_default();
            Failure::Status(response.status().as_u16(), reason)
        }
        tungstenite::HandshakeError::Failure(tungstenite::Error::Io(e)) if e.to_string().to_lowercase().contains("certificate") => Failure::Certificate(e.to_string()),
        other => Failure::Unreachable(other.to_string()),
    })?;
    // From here on reads come back after at most 10 ms, with or without a message.
    socket.get_ref().tcp().set_read_timeout(Some(Duration::from_millis(10))).map_err(|e| Failure::Unreachable(e.to_string()))?;
    Ok(socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::time::Instant;
    use tungstenite::Message;

    #[test]
    fn after_the_handshake_a_read_with_nothing_to_read_comes_back_at_once() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let mut socket = tungstenite::accept(listener.accept().unwrap().0).unwrap();
            // Quiet for a while, then one message, then the client's own.
            std::thread::sleep(Duration::from_millis(400));
            socket.send(Message::text("hello")).unwrap();
            socket.read().unwrap()
        });
        let mut socket = connect(&format!("ws://127.0.0.1:{port}/v1/listen?model=nova-3"), &[("Authorization", "Token k".into())]).unwrap();
        let asked = Instant::now();
        let quiet = socket.read();
        assert!(matches!(quiet, Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock), "{quiet:?}");
        assert!(asked.elapsed() < Duration::from_millis(200), "a quiet read waited {:?}", asked.elapsed());
        let heard = loop {
            match socket.read() {
                Ok(message) => break message,
                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("{e}"),
            }
        };
        assert_eq!(heard, Message::text("hello"));
        socket.send(Message::binary(vec![1, 2, 3, 4])).unwrap();
        assert_eq!(server.join().unwrap(), Message::binary(vec![1, 2, 3, 4]));
    }
}
