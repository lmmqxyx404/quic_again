#[cfg(unix)]
#[path = "unix.rs"]
mod imp;

pub use imp::UdpSocketState;
