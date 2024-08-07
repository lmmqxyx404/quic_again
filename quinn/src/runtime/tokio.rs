use std::{io, sync::Arc};

use super::{AsyncUdpSocket, Runtime};

/// A Quinn runtime for Tokio
#[derive(Debug)]
pub struct TokioRuntime;

impl Runtime for TokioRuntime {
    fn wrap_udp_socket(&self, sock: std::net::UdpSocket) -> io::Result<Arc<dyn AsyncUdpSocket>> {
        Ok(Arc::new(UdpSocket {}))
    }
}

#[derive(Debug)]
struct UdpSocket {}

impl AsyncUdpSocket for UdpSocket {}
