use std::io;

use crate::UdpSockRef;

/// Tokio-compatible UDP socket with some useful specializations.
///
/// Unlike a standard tokio UDP socket, this allows ECN bits to be read and written on some
/// platforms.
#[derive(Debug)]
pub struct UdpSocketState {}

impl UdpSocketState {
    pub fn new(sock: UdpSockRef<'_>) -> io::Result<Self> {
        todo!()
    }
}
