use std::{sync::Arc, time::Duration};

macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            std::task::Poll::Ready(t) => t,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }
    };
}

/// 1.
#[cfg(test)]
mod tests;

/// 4.
mod connection;
/// 2.
mod endpoint;
/// 6.
mod mutex;
/// 3.
mod runtime;
/// 5.
mod work_limiter;

#[cfg(feature = "runtime-tokio")]
pub use crate::runtime::TokioRuntime;

pub use udp;

pub use proto::{ClientConfig, ConnectionError, TransportConfig, VarInt};

pub use runtime::AsyncUdpSocket;

#[derive(Debug)]
enum ConnectionEvent {
    Close {
        error_code: VarInt,
        reason: bytes::Bytes,
    },
    Proto(proto::ConnectionEvent),
    Rebind(Arc<dyn AsyncUdpSocket>),
}

fn udp_transmit<'a>(t: &proto::Transmit, buffer: &'a [u8]) -> udp::Transmit<'a> {
    udp::Transmit {
        destination: t.destination,
        // ecn: t.ecn.map(udp_ecn),
        contents: buffer,
        // segment_size: t.segment_size,
        // src_ip: t.src_ip,
    }
}

/// Maximum number of datagrams processed in send/recv calls to make before moving on to other processing
///
/// This helps ensure we don't starve anything when the CPU is slower than the link.
/// Value is selected by picking a low number which didn't degrade throughput in benchmarks.
const IO_LOOP_BOUND: usize = 160;

/// The maximum amount of time that should be spent in `recvmsg()` calls per endpoint iteration
///
/// 50us are chosen so that an endpoint iteration with a 50us sendmsg limit blocks
/// the runtime for a maximum of about 100us.
/// Going much lower does not yield any noticeable difference, since a single `recvmmsg`
/// batch of size 32 was observed to take 30us on some systems.
const RECV_TIME_BOUND: Duration = Duration::from_micros(50);
