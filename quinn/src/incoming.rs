use std::{
    future::{Future, IntoFuture},
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use proto::{ConnectionError, ServerConfig};
use thiserror::Error;

use crate::{
    connection::{Connecting, Connection},
    endpoint::EndpointRef,
};

/// An incoming connection for which the server has not yet begun its part of the handshake
#[derive(Debug)]
pub struct Incoming(Option<State>);

impl Incoming {
    pub(crate) fn new(inner: proto::Incoming, endpoint: EndpointRef) -> Self {
        Self(Some(State { inner, endpoint }))
    }

    /// 2. Attempt to accept this incoming connection (an error may still occur)
    pub fn accept(mut self) -> Result<Connecting, ConnectionError> {
        let state = self.0.take().unwrap();
        state.endpoint.accept(state.inner, None)
    }
}

impl Drop for Incoming {
    fn drop(&mut self) {
        // Implicit reject, similar to Connection's implicit close
        if let Some(state) = self.0.take() {
            todo!() //  state.endpoint.refuse(state.inner);
        }
    }
}

#[derive(Debug)]
struct State {
    inner: proto::Incoming,
    endpoint: EndpointRef,
}

/// Error for attempting to retry an [`Incoming`] which already bears an address
/// validation token from a previous retry
#[derive(Debug, Error)]
#[error("retry() with validated Incoming")]
pub struct RetryError(Incoming);

impl RetryError {
    /// Get the [`Incoming`]
    pub fn into_incoming(self) -> Incoming {
        todo!() //  self.0
    }
}

/// Basic adapter to let [`Incoming`] be `await`-ed like a [`Connecting`]
#[derive(Debug)]
pub struct IncomingFuture(Result<Connecting, ConnectionError>);

impl Future for IncomingFuture {
    type Output = Result<Connection, ConnectionError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match &mut self.0 {
            Ok(ref mut connecting) => Pin::new(connecting).poll(cx),
            Err(e) => Poll::Ready(Err(e.clone())),
        }
    }
}

impl IntoFuture for Incoming {
    type Output = Result<Connection, ConnectionError>;
    type IntoFuture = IncomingFuture;

    fn into_future(self) -> Self::IntoFuture {
        IncomingFuture(self.accept())
    }
}
