use std::{io, sync::Arc};

use super::{AsyncUdpSocket, Runtime};

/// A Quinn runtime for Tokio
#[derive(Debug)]
pub struct TokioRuntime;

impl Runtime for TokioRuntime {
    fn wrap_udp_socket(&self, sock: std::net::UdpSocket) -> io::Result<Arc<dyn AsyncUdpSocket>> {
        Ok(Arc::new(UdpSocket {
            io: tokio::net::UdpSocket::from_std(sock)?,
        }))
    }
}

#[derive(Debug)]
struct UdpSocket {
    io: tokio::net::UdpSocket,
}

impl AsyncUdpSocket for UdpSocket {
    fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.io.local_addr()
    }
}
