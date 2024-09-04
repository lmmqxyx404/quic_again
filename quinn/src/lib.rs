
use std::time::Duration;

/// 1.
#[cfg(test)]
mod tests;

/// 4.
mod connection;
/// 2.
mod endpoint;
/// 3.
mod runtime;
/// 5.
mod work_limiter;


#[cfg(feature = "runtime-tokio")]
pub use crate::runtime::TokioRuntime;

pub use proto::{ClientConfig, ConnectionError, TransportConfig};

/// The maximum amount of time that should be spent in `recvmsg()` calls per endpoint iteration
///
/// 50us are chosen so that an endpoint iteration with a 50us sendmsg limit blocks
/// the runtime for a maximum of about 100us.
/// Going much lower does not yield any noticeable difference, since a single `recvmmsg`
/// batch of size 32 was observed to take 30us on some systems.
const RECV_TIME_BOUND: Duration = Duration::from_micros(50);
