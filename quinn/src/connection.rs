use std::{
    future::Future,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use tokio::sync::{futures::Notified, mpsc, oneshot, Notify};
use tracing::{Instrument, Span};

use crate::{runtime::Runtime, AsyncUdpSocket, ConnectionEvent};

use proto::{
    congestion::Controller, ConnectionError, ConnectionHandle, ConnectionStats, Dir, EndpointEvent,
    StreamEvent, StreamId,
};

/// In-progress connection attempt future
/// todo: tag
#[derive(Debug)]
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
pub struct Connecting {}

impl Connecting {
    pub(crate) fn new(
        handle: ConnectionHandle,
        conn: proto::Connection,
        endpoint_events: mpsc::UnboundedSender<(ConnectionHandle, EndpointEvent)>,
        conn_events: mpsc::UnboundedReceiver<ConnectionEvent>,
        socket: Arc<dyn AsyncUdpSocket>,
        runtime: Arc<dyn Runtime>,
    ) -> Self {
        let (on_handshake_data_send, on_handshake_data_recv) = oneshot::channel();
        let (on_connected_send, on_connected_recv) = oneshot::channel();
        let conn = ConnectionRef::new(
            handle,
            conn,
            endpoint_events,
            conn_events,
            on_handshake_data_send,
            on_connected_send,
            socket,
            runtime.clone(),
        );

        let driver = ConnectionDriver(conn.clone());

        todo!()
    }
}

impl Future for Connecting {
    type Output = Result<Connection, ConnectionError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        todo!()
    }
}

/// A QUIC connection.
///
/// If all references to a connection (including every clone of the `Connection` handle, streams of
/// incoming streams, and the various stream types) have been dropped, then the connection will be
/// automatically closed with an `error_code` of 0 and an empty `reason`. You can also close the
/// connection explicitly by calling [`Connection::close()`].
///
/// May be cloned to obtain another handle to the same connection.
///
/// [`Connection::close()`]: Connection::close
#[derive(Debug, Clone)]
pub struct Connection(ConnectionRef);

#[derive(Debug)]
pub(crate) struct ConnectionRef(Arc<ConnectionInner>);

impl ConnectionRef {
    #[allow(clippy::too_many_arguments)]
    fn new(
        handle: ConnectionHandle,
        conn: proto::Connection,
        endpoint_events: mpsc::UnboundedSender<(ConnectionHandle, EndpointEvent)>,
        conn_events: mpsc::UnboundedReceiver<ConnectionEvent>,
        on_handshake_data: oneshot::Sender<()>,
        on_connected: oneshot::Sender<bool>,
        socket: Arc<dyn AsyncUdpSocket>,
        runtime: Arc<dyn Runtime>,
    ) -> Self {
        Self(Arc::new(ConnectionInner {}))
    }
}

impl Clone for ConnectionRef {
    fn clone(&self) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct ConnectionInner {
    // pub(crate) state: Mutex<State>,
}

/// A future that drives protocol logic for a connection
///
/// This future handles the protocol logic for a single connection, routing events from the
/// `Connection` API object to the `Endpoint` task and the related stream-related interfaces.
/// It also keeps track of outstanding timeouts for the `Connection`.
///
/// If the connection encounters an error condition, this future will yield an error. It will
/// terminate (yielding `Ok(())`) if the connection was closed without error. Unlike other
/// connection-related futures, this waits for the draining period to complete to ensure that
/// packets still in flight from the peer are handled gracefully.
#[must_use = "connection drivers must be spawned for their connections to function"]
#[derive(Debug)]
struct ConnectionDriver(ConnectionRef);
