use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use proto::{Connection, ConnectionError};

/// In-progress connection attempt future
/// todo: tag
#[derive(Debug)]
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
pub struct Connecting {}

impl Future for Connecting {
    type Output = Result<Connection, ConnectionError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        todo!()
    }
}
