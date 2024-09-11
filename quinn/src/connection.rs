use std::{
    fmt,
    future::Future,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::Instant,
};

use bytes::Bytes;
use pin_project_lite::pin_project;
use rustc_hash::FxHashMap;
use tokio::sync::{futures::Notified, mpsc, oneshot, Notify};
use tracing::{debug_span, Instrument, Span};

use crate::{
    mutex::Mutex,
    recv_stream::RecvStream,
    runtime::{AsyncTimer, Runtime, UdpPoller},
    send_stream::SendStream,
    udp_transmit, AsyncUdpSocket, ConnectionEvent, VarInt,
};

use proto::{
    congestion::Controller, ConnectionError, ConnectionHandle, ConnectionStats, Dir, EndpointEvent,
    StreamEvent, StreamId,
};

/// In-progress connection attempt future
/// todo: tag
#[derive(Debug)]
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
pub struct Connecting {
    conn: Option<ConnectionRef>,
    connected: oneshot::Receiver<bool>,
}

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
        runtime.spawn(Box::pin(
            async {
                if let Err(e) = driver.await {
                    tracing::error!("I/O error: {e}");
                }
            }
            .instrument(Span::current()),
        ));

        Self {
            conn: Some(conn),
            connected: on_connected_recv,
        }
    }
    /// Convert into a 0-RTT or 0.5-RTT connection at the cost of weakened security
    ///
    /// Returns `Ok` immediately if the local endpoint is able to attempt sending 0/0.5-RTT data.
    /// If so, the returned [`Connection`] can be used to send application data without waiting for
    /// the rest of the handshake to complete, at the cost of weakened cryptographic security
    /// guarantees. The returned [`ZeroRttAccepted`] future resolves when the handshake does
    /// complete, at which point subsequently opened streams and written data will have full
    /// cryptographic protection.
    ///
    /// ## Outgoing
    ///
    /// For outgoing connections, the initial attempt to convert to a [`Connection`] which sends
    /// 0-RTT data will proceed if the [`crypto::ClientConfig`][crate::crypto::ClientConfig]
    /// attempts to resume a previous TLS session. However, **the remote endpoint may not actually
    /// _accept_ the 0-RTT data**--yet still accept the connection attempt in general. This
    /// possibility is conveyed through the [`ZeroRttAccepted`] future--when the handshake
    /// completes, it resolves to true if the 0-RTT data was accepted and false if it was rejected.
    /// If it was rejected, the existence of streams opened and other application data sent prior
    /// to the handshake completing will not be conveyed to the remote application, and local
    /// operations on them will return `ZeroRttRejected` errors.
    ///
    /// A server may reject 0-RTT data at its discretion, but accepting 0-RTT data requires the
    /// relevant resumption state to be stored in the server, which servers may limit or lose for
    /// various reasons including not persisting resumption state across server restarts.
    ///
    /// If manually providing a [`crypto::ClientConfig`][crate::crypto::ClientConfig], check your
    /// implementation's docs for 0-RTT pitfalls.
    ///
    /// ## Incoming
    ///
    /// For incoming connections, conversion to 0.5-RTT will always fully succeed. `into_0rtt` will
    /// always return `Ok` and the [`ZeroRttAccepted`] will always resolve to true.
    ///
    /// If manually providing a [`crypto::ServerConfig`][crate::crypto::ServerConfig], check your
    /// implementation's docs for 0-RTT pitfalls.
    ///
    /// ## Security
    ///
    /// On outgoing connections, this enables transmission of 0-RTT data, which is vulnerable to
    /// replay attacks, and should therefore never invoke non-idempotent operations.
    ///
    /// On incoming connections, this enables transmission of 0.5-RTT data, which may be sent
    /// before TLS client authentication has occurred, and should therefore not be used to send
    /// data for which client authentication is being used.
    pub fn into_0rtt(mut self) -> Result<(Connection, ZeroRttAccepted), Self> {
        // This lock borrows `self` and would normally be dropped at the end of this scope, so we'll
        // have to release it explicitly before returning `self` by value.
        let conn = (self.conn.as_mut().unwrap()).state.lock("into_0rtt");

        let is_ok = conn.inner.has_0rtt() || conn.inner.side().is_server();
        drop(conn);
        if is_ok {
            let conn = self.conn.take().unwrap();
            Ok((Connection(conn), ZeroRttAccepted(self.connected)))
        } else {
            Err(self)
        }
    }
}

impl Future for Connecting {
    type Output = Result<Connection, ConnectionError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.connected).poll(cx).map(|_| {
            let conn = self.conn.take().unwrap();
            let inner = conn.state.lock("connecting");
            if inner.connected {
                drop(inner);
                Ok(Connection(conn))
            } else {
                Err(inner
                    .error
                    .clone()
                    .expect("connected signaled without connection success or error"))
            }
        })
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

impl Connection {
    /// Initiate a new outgoing unidirectional stream.
    ///
    /// Streams are cheap and instantaneous to open unless blocked by flow control. As a
    /// consequence, the peer won't be notified that a stream has been opened until the stream is
    /// actually used.
    pub fn open_uni(&self) -> OpenUni<'_> {
        OpenUni {
            conn: &self.0,
            notify: self.0.shared.stream_budget_available[Dir::Uni as usize].notified(),
        }
    }
    /// Accept the next incoming uni-directional stream
    pub fn accept_uni(&self) -> AcceptUni<'_> {
        AcceptUni {
            conn: &self.0,
            notify: self.0.shared.stream_incoming[Dir::Uni as usize].notified(),
        }
    }

    /// Derive keying material from this connection's TLS session secrets.
    ///
    /// When both peers call this method with the same `label` and `context`
    /// arguments and `output` buffers of equal length, they will get the
    /// same sequence of bytes in `output`. These bytes are cryptographically
    /// strong and pseudorandom, and are suitable for use as keying material.
    ///
    /// See [RFC5705](https://tools.ietf.org/html/rfc5705) for more information.
    pub fn export_keying_material(
        &self,
        output: &mut [u8],
        label: &[u8],
        context: &[u8],
    ) -> Result<(), proto::crypto::ExportKeyingMaterialError> {
        self.0
            .state
            .lock("export_keying_material")
            .inner
            .crypto_session()
            .export_keying_material(output, label, context)
    }
}

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
        Self(Arc::new(ConnectionInner {
            state: Mutex::new(State {
                ref_count: 0,
                handle,
                error: None,
                connected: false,
                on_connected: Some(on_connected),
                inner: conn,
                runtime,

                driver: None,
                on_handshake_data: Some(on_handshake_data),
                blocked_writers: FxHashMap::default(),
                blocked_readers: FxHashMap::default(),
                stopped: FxHashMap::default(),

                endpoint_events,

                io_poller: socket.clone().create_io_poller(),
                conn_events,
                socket,
                send_buffer: Vec::new(),
                buffered_transmit: None,

                timer: None,
                timer_deadline: None,
            }),
            shared: Shared::default(),
        }))
    }
}

impl Clone for ConnectionRef {
    fn clone(&self) -> Self {
        self.state.lock("clone").ref_count += 1;
        Self(self.0.clone())
    }
}

impl Drop for ConnectionRef {
    fn drop(&mut self) {
        let conn = &mut *self.state.lock("drop");
        if let Some(x) = conn.ref_count.checked_sub(1) {
            conn.ref_count = x;
            if x == 0 && !conn.inner.is_closed() {
                // If the driver is alive, it's just it and us, so we'd better shut it down. If it's
                // not, we can't do any harm. If there were any streams being opened, then either
                // the connection will be closed for an unrelated reason or a fresh reference will
                // be constructed for the newly opened stream.
                conn.implicit_close(&self.shared);
            }
        }
    }
}

impl std::ops::Deref for ConnectionRef {
    type Target = ConnectionInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub(crate) struct ConnectionInner {
    pub(crate) state: Mutex<State>,
    shared: Shared,
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

impl Future for ConnectionDriver {
    type Output = Result<(), io::Error>;

    #[allow(unused_mut)] // MSRV
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let conn = &mut *self.0.state.lock("poll");

        let span = debug_span!("drive", id = conn.handle.0);
        let _guard = span.enter();

        if let Err(e) = conn.process_conn_events(&self.0.shared, cx) {
            todo!() // conn.terminate(e, &self.0.shared);
                    // return Poll::Ready(Ok(()));
        }

        let mut keep_going = conn.drive_transmit(cx)?;
        // If a timer expires, there might be more to transmit. When we transmit something, we
        // might need to reset a timer. Hence, we must loop until neither happens.
        keep_going |= conn.drive_timer(cx);
        conn.forward_endpoint_events();
        conn.forward_app_events(&self.0.shared);
        if !conn.inner.is_drained() {
            if keep_going {
                // If the connection hasn't processed all tasks, schedule it again
                cx.waker().wake_by_ref();
            } else {
                conn.driver = Some(cx.waker().clone());
            }
            return Poll::Pending;
        }

        if conn.error.is_none() {
            unreachable!("drained connections always have an error");
        }
        Poll::Ready(Ok(()))
    }
}

pub(crate) struct State {
    /// Number of live handles that can be used to initiate or handle I/O; excludes the driver
    ref_count: usize,
    handle: ConnectionHandle,
    connected: bool,
    /// Always set to Some before the connection becomes drained
    pub(crate) error: Option<ConnectionError>,
    on_connected: Option<oneshot::Sender<bool>>,
    pub(crate) inner: proto::Connection,
    runtime: Arc<dyn Runtime>,
    driver: Option<Waker>,
    on_handshake_data: Option<oneshot::Sender<()>>,
    pub(crate) blocked_writers: FxHashMap<StreamId, Waker>,
    pub(crate) blocked_readers: FxHashMap<StreamId, Waker>,
    pub(crate) stopped: FxHashMap<StreamId, Waker>,
    endpoint_events: mpsc::UnboundedSender<(ConnectionHandle, EndpointEvent)>,
    conn_events: mpsc::UnboundedReceiver<ConnectionEvent>,
    socket: Arc<dyn AsyncUdpSocket>,
    send_buffer: Vec<u8>,
    /// We buffer a transmit when the underlying I/O would block
    buffered_transmit: Option<proto::Transmit>,
    io_poller: Pin<Box<dyn UdpPoller>>,
    timer: Option<Pin<Box<dyn AsyncTimer>>>,
    timer_deadline: Option<Instant>,
}

impl State {
    /// Close for a reason other than the application's explicit request
    pub(crate) fn implicit_close(&mut self, shared: &Shared) {
        self.close(0u32.into(), Bytes::new(), shared);
    }

    fn close(&mut self, error_code: VarInt, reason: Bytes, shared: &Shared) {
        self.inner.close(self.runtime.now(), error_code, reason);
        self.terminate(ConnectionError::LocallyClosed, shared);
        self.wake();
    }

    /// Used to wake up all blocked futures when the connection becomes closed for any reason
    fn terminate(&mut self, reason: ConnectionError, shared: &Shared) {
        self.error = Some(reason.clone());
        if let Some(x) = self.on_handshake_data.take() {
            let _ = x.send(());
        }
        wake_all(&mut self.blocked_writers);
        wake_all(&mut self.blocked_readers);
        shared.stream_budget_available[Dir::Uni as usize].notify_waiters();
        shared.stream_budget_available[Dir::Bi as usize].notify_waiters();
        shared.stream_incoming[Dir::Uni as usize].notify_waiters();
        shared.stream_incoming[Dir::Bi as usize].notify_waiters();
        shared.datagram_received.notify_waiters();
        shared.datagrams_unblocked.notify_waiters();
        if let Some(x) = self.on_connected.take() {
            let _ = x.send(false);
        }
        wake_all(&mut self.stopped);
        shared.closed.notify_waiters();
    }

    /// Wake up a blocked `Driver` task to process I/O
    pub(crate) fn wake(&mut self) {
        if let Some(x) = self.driver.take() {
            x.wake();
        }
    }

    /// If this returns `Err`, the endpoint is dead, so the driver should exit immediately.
    fn process_conn_events(
        &mut self,
        shared: &Shared,
        cx: &mut Context,
    ) -> Result<(), ConnectionError> {
        loop {
            match self.conn_events.poll_recv(cx) {
                Poll::Ready(Some(ConnectionEvent::Rebind(socket))) => {
                    todo!() //
                            /*  self.socket = socket;
                            self.io_poller = self.socket.clone().create_io_poller();
                            self.inner.local_address_changed(); */
                }
                Poll::Ready(Some(ConnectionEvent::Proto(event))) => {
                    self.inner.handle_event(event);
                }
                Poll::Ready(Some(ConnectionEvent::Close { reason, error_code })) => {
                    self.close(error_code, reason, shared);
                }
                Poll::Ready(None) => {
                    todo!()
                    /*  return Err(ConnectionError::TransportError(proto::TransportError {
                        code: proto::TransportErrorCode::INTERNAL_ERROR,
                        frame: None,
                        reason: "endpoint driver future was dropped".to_string(),
                    })); */
                }
                Poll::Pending => {
                    return Ok(());
                }
            }
        }
    }

    fn drive_transmit(&mut self, cx: &mut Context) -> io::Result<bool> {
        let now = self.runtime.now();
        let mut transmits = 0;

        let max_datagrams = self.socket.max_transmit_segments();
        loop {
            // Retry the last transmit, or get a new one.
            let t = match self.buffered_transmit.take() {
                Some(t) => t,
                None => {
                    self.send_buffer.clear();
                    self.send_buffer.reserve(self.inner.current_mtu() as usize);
                    match self
                        .inner
                        .poll_transmit(now, max_datagrams, &mut self.send_buffer)
                    {
                        Some(t) => {
                            transmits += match t.segment_size {
                                None => 1,
                                Some(s) => (t.size + s - 1) / s, // round up
                            };
                            t
                        }
                        None => break,
                    }
                }
            };

            if self.io_poller.as_mut().poll_writable(cx)?.is_pending() {
                // Retry after a future wakeup
                self.buffered_transmit = Some(t);
                return Ok(false);
            }

            let len = t.size;
            let retry = match self
                .socket
                .try_send(&udp_transmit(&t, &self.send_buffer[..len]))
            {
                Ok(()) => false,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => true,
                Err(e) => return Err(e),
            };
            if retry {
                // We thought the socket was writable, but it wasn't. Retry so that either another
                // `poll_writable` call determines that the socket is indeed not writable and
                // registers us for a wakeup, or the send succeeds if this really was just a
                // transient failure.
                // self.buffered_transmit = Some(t);
                // continue;
                todo!()
            }
            if transmits >= MAX_TRANSMIT_DATAGRAMS {
                // TODO: What isn't ideal here yet is that if we don't poll all
                // datagrams that could be sent we don't go into the `app_limited`
                // state and CWND continues to grow until we get here the next time.
                // See https://github.com/quinn-rs/quinn/issues/1126
                todo!() // return Ok(true);
            }
        }

        Ok(false)
    }

    fn drive_timer(&mut self, cx: &mut Context) -> bool {
        // Check whether we need to (re)set the timer. If so, we must poll again to ensure the
        // timer is registered with the runtime (and check whether it's already
        // expired).
        match self.inner.poll_timeout() {
            Some(deadline) => {
                if let Some(delay) = &mut self.timer {
                    // There is no need to reset the tokio timer if the deadline
                    // did not change
                    if self
                        .timer_deadline
                        .map(|current_deadline| current_deadline != deadline)
                        .unwrap_or(true)
                    {
                        delay.as_mut().reset(deadline);
                    }
                } else {
                    self.timer = Some(self.runtime.new_timer(deadline));
                }
                // Store the actual expiration time of the timer
                self.timer_deadline = Some(deadline);
            }
            None => {
                self.timer_deadline = None;
                return false;
            }
        }

        if self.timer_deadline.is_none() {
            return false;
        }

        let delay = self
            .timer
            .as_mut()
            .expect("timer must exist in this state")
            .as_mut();
        if delay.poll(cx).is_pending() {
            // Since there wasn't a timeout event, there is nothing new
            // for the connection to do
            return false;
        }

        // A timer expired, so the caller needs to check for
        // new transmits, which might cause new timers to be set.
        self.inner.handle_timeout(self.runtime.now());
        self.timer_deadline = None;
        true
    }
    fn forward_endpoint_events(&mut self) {
        while let Some(event) = self.inner.poll_endpoint_events() {
            // If the endpoint driver is gone, noop.
            let _ = self.endpoint_events.send((self.handle, event));
        }
    }

    fn forward_app_events(&mut self, shared: &Shared) {
        while let Some(event) = self.inner.poll() {
            use proto::Event::*;
            match event {
                HandshakeDataReady => {
                    if let Some(x) = self.on_handshake_data.take() {
                        let _ = x.send(());
                    }
                }
                Connected => {
                    self.connected = true;
                    if let Some(x) = self.on_connected.take() {
                        // We don't care if the on-connected future was dropped
                        let _ = x.send(self.inner.accepted_0rtt());
                    }
                    if self.inner.side().is_client() && !self.inner.accepted_0rtt() {
                        // Wake up rejected 0-RTT streams so they can fail immediately with
                        // `ZeroRttRejected` errors.
                        wake_all(&mut self.blocked_writers);
                        wake_all(&mut self.blocked_readers);
                        wake_all(&mut self.stopped);
                    }
                }
                ConnectionLost { reason } => {
                    self.terminate(reason, shared);
                }
                Stream(StreamEvent::Writable { id }) => {
                    todo!()
                    // wake_stream(id, &mut self.blocked_writers),
                }
                Stream(StreamEvent::Opened { dir: Dir::Uni }) => {
                    shared.stream_incoming[Dir::Uni as usize].notify_waiters();
                }
                Stream(StreamEvent::Opened { dir: Dir::Bi }) => {
                    shared.stream_incoming[Dir::Bi as usize].notify_waiters();
                }
                DatagramReceived => {
                    shared.datagram_received.notify_waiters();
                }
                DatagramsUnblocked => {
                    shared.datagrams_unblocked.notify_waiters();
                }
                Stream(StreamEvent::Readable { id }) => {
                    todo!()
                    //wake_stream(id, &mut self.blocked_readers),
                }
                Stream(StreamEvent::Available { dir }) => {
                    // Might mean any number of streams are ready, so we wake up everyone
                    shared.stream_budget_available[dir as usize].notify_waiters();
                }
                Stream(StreamEvent::Finished { id }) => wake_stream(id, &mut self.stopped),
                Stream(StreamEvent::Stopped { id, .. }) => {
                    todo!()
                }
            }
        }
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        todo!() // f.debug_struct("State").field("inner", &self.inner).finish()
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if !self.inner.is_drained() {
            // Ensure the endpoint can tidy up
            let _ = self
                .endpoint_events
                .send((self.handle, proto::EndpointEvent::drained()));
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Shared {
    /// Notified when new streams may be locally initiated due to an increase in stream ID flow
    /// control budget
    stream_budget_available: [Notify; 2],
    /// Notified when the peer has initiated a new stream
    stream_incoming: [Notify; 2],
    datagram_received: Notify,
    datagrams_unblocked: Notify,
    closed: Notify,
}

fn wake_all(wakers: &mut FxHashMap<StreamId, Waker>) {
    wakers.drain().for_each(|(_, waker)| waker.wake())
}

pin_project! {
    /// Future produced by [`Connection::open_uni`]
    pub struct OpenUni<'a> {
        conn: &'a ConnectionRef,
        #[pin]
        notify: Notified<'a>,
    }
}

impl Future for OpenUni<'_> {
    type Output = Result<SendStream, ConnectionError>;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let (conn, id, is_0rtt) = ready!(poll_open(ctx, this.conn, this.notify, Dir::Uni))?;
        Poll::Ready(Ok(SendStream::new(conn, id, is_0rtt)))
    }
}

fn poll_open<'a>(
    ctx: &mut Context<'_>,
    conn: &'a ConnectionRef,
    mut notify: Pin<&mut Notified<'a>>,
    dir: Dir,
) -> Poll<Result<(ConnectionRef, StreamId, bool), ConnectionError>> {
    let mut state = conn.state.lock("poll_open");
    if let Some(ref e) = state.error {
        todo!() // return Poll::Ready(Err(e.clone()));
    } else if let Some(id) = state.inner.streams().open(dir) {
        let is_0rtt = state.inner.side().is_client() && state.inner.is_handshaking();
        drop(state); // Release the lock so clone can take it
        return Poll::Ready(Ok((conn.clone(), id, is_0rtt)));
    }
    todo!()
}

fn wake_stream(stream_id: StreamId, wakers: &mut FxHashMap<StreamId, Waker>) {
    if let Some(waker) = wakers.remove(&stream_id) {
        waker.wake();
    }
}

pin_project! {
    /// Future produced by [`Connection::accept_uni`]
    pub struct AcceptUni<'a> {
        conn: &'a ConnectionRef,
        #[pin]
        notify: Notified<'a>,
    }
}

impl Future for AcceptUni<'_> {
    type Output = Result<RecvStream, ConnectionError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let (conn, id, is_0rtt) = ready!(poll_accept(ctx, this.conn, this.notify, Dir::Uni))?;
        Poll::Ready(Ok(RecvStream::new(conn, id, is_0rtt)))
    }
}

fn poll_accept<'a>(
    ctx: &mut Context<'_>,
    conn: &'a ConnectionRef,
    mut notify: Pin<&mut Notified<'a>>,
    dir: Dir,
) -> Poll<Result<(ConnectionRef, StreamId, bool), ConnectionError>> {
    let mut state = conn.state.lock("poll_accept");
    // Check for incoming streams before checking `state.error` so that already-received streams,
    // which are necessarily finite, can be drained from a closed connection.
    if let Some(id) = state.inner.streams().accept(dir) {
        let is_0rtt = state.inner.is_handshaking();
        state.wake(); // To send additional stream ID credit
        drop(state); // Release the lock so clone can take it
        return Poll::Ready(Ok((conn.clone(), id, is_0rtt)));
    } else if let Some(ref e) = state.error {
        todo!() // return Poll::Ready(Err(e.clone()));
    }
    loop {
        match notify.as_mut().poll(ctx) {
            // `state` lock ensures we didn't race with readiness
            Poll::Pending => return Poll::Pending,
            // Spurious wakeup, get a new future
            Poll::Ready(()) => {
                todo!() // notify.set(conn.shared.stream_incoming[dir as usize].notified()),
            }
        }
    }
}

/// Future that completes when a connection is fully established
///
/// For clients, the resulting value indicates if 0-RTT was accepted. For servers, the resulting
/// value is meaningless.
#[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
pub struct ZeroRttAccepted(oneshot::Receiver<bool>);

impl Future for ZeroRttAccepted {
    type Output = bool;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|x| x.unwrap_or(false))
    }
}

/// The maximum amount of datagrams which will be produced in a single `drive_transmit` call
///
/// This limits the amount of CPU resources consumed by datagram generation,
/// and allows other tasks (like receiving ACKs) to run in between.
const MAX_TRANSMIT_DATAGRAMS: usize = 20;
