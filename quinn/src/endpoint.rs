use bytes::Bytes;
use proto::{
    ClientConfig, ConnectError, ConnectionHandle, EndpointConfig, EndpointEvent, ServerConfig,
    VarInt,
};
use rustc_hash::FxHashMap;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    collections::VecDeque,
    future::Future,
    io::{self, IoSliceMut},
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    time::Instant,
};
use tokio::sync::{mpsc, Notify};
use tracing::{Instrument, Span};
use udp::{RecvMeta, BATCH_SIZE};

#[cfg(feature = "ring")]
use crate::runtime::default_runtime;
use crate::{
    connection::Connecting,
    runtime::{AsyncUdpSocket, Runtime},
    work_limiter::WorkLimiter,
    ConnectionEvent, IO_LOOP_BOUND, RECV_TIME_BOUND,
};

/// A QUIC endpoint.
///
/// An endpoint corresponds to a single UDP socket, may host many connections, and may act as both
/// client and server for different connections.
///
/// May be cloned to obtain another handle to the same endpoint.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub(crate) inner: EndpointRef,
    runtime: Arc<dyn Runtime>,
}

impl Endpoint {
    /// Helper to construct an endpoint for use with outgoing connections only
    ///
    /// Note that `addr` is the *local* address to bind to, which should usually be a wildcard
    /// address like `0.0.0.0:0` or `[::]:0`, which allow communication with any reachable IPv4 or
    /// IPv6 address respectively from an OS-assigned port.
    ///
    /// If an IPv6 address is provided, attempts to make the socket dual-stack so as to allow
    /// communication with both IPv4 and IPv6 addresses. As such, calling `Endpoint::client` with
    /// the address `[::]:0` is a reasonable default to maximize the ability to connect to other
    /// address. For example:
    ///
    /// ```
    /// quinn::Endpoint::client((std::net::Ipv6Addr::UNSPECIFIED, 0).into());
    /// ```
    ///
    /// Some environments may not allow creation of dual-stack sockets, in which case an IPv6
    /// client will only be able to connect to IPv6 servers. An IPv4 client is never dual-stack.
    #[cfg(feature = "ring")]
    pub fn client(addr: SocketAddr) -> io::Result<Self> {
        let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
        if addr.is_ipv6() {
            if let Err(e) = socket.set_only_v6(false) {
                tracing::debug!(%e, "unable to make socket dual-stack");
            }
        }
        socket.bind(&addr.into())?;
        let runtime = default_runtime()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no async runtime found"))?;
        Self::new_with_abstract_socket(
            EndpointConfig::default(),
            None,
            runtime.wrap_udp_socket(socket.into())?,
            runtime,
        )
    }
    /// Construct an endpoint with arbitrary configuration and pre-constructed abstract socket
    ///
    /// Useful when `socket` has additional state (e.g. sidechannels) attached for which shared
    /// ownership is needed.
    pub fn new_with_abstract_socket(
        config: EndpointConfig,
        server_config: Option<ServerConfig>,
        socket: Arc<dyn AsyncUdpSocket>,
        runtime: Arc<dyn Runtime>,
    ) -> io::Result<Self> {
        let addr = socket.local_addr()?;
        let allow_mtud = !socket.may_fragment();
        let rc = EndpointRef::new(
            socket,
            proto::Endpoint::new(
                Arc::new(config),
                server_config.map(Arc::new),
                allow_mtud,
                None,
            ),
            addr.is_ipv6(),
            runtime.clone(),
        );
        let driver = EndpointDriver(rc.clone());
        runtime.spawn(Box::pin(
            async {
                if let Err(e) = driver.await {
                    tracing::error!("I/O error: {}", e);
                }
            }
            .instrument(Span::current()),
        ));
        Ok(Self { inner: rc, runtime })
    }
    /// Connect to a remote endpoint using a custom configuration.
    ///
    /// See [`connect()`] for details.
    ///
    /// [`connect()`]: Endpoint::connect
    pub fn connect_with(
        &self,
        config: ClientConfig,
        addr: SocketAddr,
        server_name: &str,
    ) -> Result<Connecting, ConnectError> {
        let mut endpoint = self.inner.state.lock().unwrap();
        if endpoint.driver_lost || endpoint.recv_state.connections.close.is_some() {
            return Err(ConnectError::EndpointStopping);
        }
        if addr.is_ipv6() && !endpoint.ipv6 {
            return Err(ConnectError::InvalidRemoteAddress(addr));
        }

        let addr = if endpoint.ipv6 {
            todo!() // SocketAddr::V6(ensure_ipv6(addr))
        } else {
            addr
        };

        let (ch, conn) = endpoint
            .inner
            .connect(self.runtime.now(), config, addr, server_name)?;

        let socket = endpoint.socket.clone();
        endpoint.stats.outgoing_handshakes += 1;
        Ok(endpoint
            .recv_state
            .connections
            .insert(ch, conn, socket, self.runtime.clone()))
    }
}

#[derive(Debug)]
pub(crate) struct EndpointRef(Arc<EndpointInner>);

impl EndpointRef {
    pub(crate) fn new(
        socket: Arc<dyn AsyncUdpSocket>,
        inner: proto::Endpoint,
        ipv6: bool,
        runtime: Arc<dyn Runtime>,
    ) -> Self {
        let (sender, events) = mpsc::unbounded_channel();
        let recv_state = RecvState::new(sender, socket.max_receive_segments(), &inner);
        Self(Arc::new(EndpointInner {
            shared: Shared {
                incoming: Notify::new(),
                idle: Notify::new(),
            },
            state: Mutex::new(State {
                ref_count: 0,
                driver: None,
                runtime,
                driver_lost: false,
                recv_state,
                ipv6,
                prev_socket: None,
                socket,
                inner,
                events,
                stats: EndpointStats::default(),
            }),
        }))
    }
}

impl Clone for EndpointRef {
    fn clone(&self) -> Self {
        self.0.state.lock().unwrap().ref_count += 1;
        Self(self.0.clone())
    }
}

impl Drop for EndpointRef {
    fn drop(&mut self) {
        let endpoint = &mut *self.0.state.lock().unwrap();
        if let Some(x) = endpoint.ref_count.checked_sub(1) {
            endpoint.ref_count = x;
            if x == 0 {
                // If the driver is about to be on its own, ensure it can shut down if the last
                // connection is gone.
                if let Some(task) = endpoint.driver.take() {
                    task.wake();
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct EndpointInner {
    /// 1
    pub(crate) state: Mutex<State>,
    /// 2.
    pub(crate) shared: Shared,
}

impl std::ops::Deref for EndpointRef {
    type Target = EndpointInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// State directly involved in handling incoming packets
#[derive(Debug)]
struct RecvState {
    /// 1
    connections: ConnectionSet,
    /// 2
    incoming: VecDeque<proto::Incoming>,
    /// 3
    recv_limiter: WorkLimiter,
    /// 4.
    recv_buf: Box<[u8]>,
}

impl RecvState {
    fn new(
        sender: mpsc::UnboundedSender<(ConnectionHandle, EndpointEvent)>,
        max_receive_segments: usize,
        endpoint: &proto::Endpoint,
    ) -> Self {
        let recv_buf = vec![
            0;
            endpoint.config().get_max_udp_payload_size().min(64 * 1024) as usize
                * max_receive_segments
                * BATCH_SIZE
        ];
        Self {
            connections: ConnectionSet {
                close: None,
                senders: FxHashMap::default(),
                sender,
            },
            incoming: VecDeque::new(),
            recv_limiter: WorkLimiter::new(RECV_TIME_BOUND),
            recv_buf: recv_buf.into(),
        }
    }

    fn poll_socket(
        &mut self,
        cx: &mut Context,
        endpoint: &mut proto::Endpoint,
        socket: &dyn AsyncUdpSocket,
        runtime: &dyn Runtime,
        now: Instant,
    ) -> Result<PollProgress, io::Error> {
        let mut received_connection_packet = false;
        let mut metas = [RecvMeta::default(); BATCH_SIZE];
        let mut iovs: [IoSliceMut; BATCH_SIZE] = {
            let mut bufs = self
                .recv_buf
                .chunks_mut(self.recv_buf.len() / BATCH_SIZE)
                .map(IoSliceMut::new);

            // expect() safe as self.recv_buf is chunked into BATCH_SIZE items
            // and iovs will be of size BATCH_SIZE, thus from_fn is called
            // exactly BATCH_SIZE times.
            std::array::from_fn(|_| bufs.next().expect("BATCH_SIZE elements"))
        };

        loop {
            match socket.poll_recv(cx, &mut iovs, &mut metas) {
                Poll::Ready(Ok(msgs)) => {
                    todo!()
                }
                Poll::Pending => {
                    return Ok(PollProgress {
                        received_connection_packet,
                        keep_going: false,
                    });
                }
                // Ignore ECONNRESET as it's undefined in QUIC and may be injected by an
                // attacker
                Poll::Ready(Err(ref e)) if e.kind() == io::ErrorKind::ConnectionReset => {
                    todo!()
                }
                Poll::Ready(Err(e)) => {
                    todo!() // return Err(e);
                }
            }
            todo!()
        }
    }
}

#[derive(Debug)]
pub(crate) struct State {
    /// 1. Number of live handles that can be used to initiate or handle I/O; excludes the driver
    ref_count: usize,
    /// 2.
    driver: Option<Waker>,
    /// 3.
    runtime: Arc<dyn Runtime>,
    /// 4.
    driver_lost: bool,
    /// 5.
    recv_state: RecvState,
    /// 6. support or not
    ipv6: bool,
    /// 7. During an active migration, abandoned_socket receives traffic
    /// until the first packet arrives on the new socket.
    prev_socket: Option<Arc<dyn AsyncUdpSocket>>,
    /// 8.
    inner: proto::Endpoint,
    /// 9.
    socket: Arc<dyn AsyncUdpSocket>,
    /// 10.
    events: mpsc::UnboundedReceiver<(ConnectionHandle, EndpointEvent)>,
    /// 11.
    stats: EndpointStats,
}

impl Drop for State {
    fn drop(&mut self) {
        for incoming in self.recv_state.incoming.drain(..) {
            todo!() // self.inner.ignore(incoming);
        }
    }
}

#[derive(Debug)]
pub(crate) struct Shared {
    incoming: Notify,
    idle: Notify,
}

impl State {
    /// 1.
    fn drive_recv(&mut self, cx: &mut Context, now: Instant) -> Result<bool, io::Error> {
        let get_time = || self.runtime.now();
        self.recv_state.recv_limiter.start_cycle(get_time);
        if let Some(socket) = &self.prev_socket {
            // We don't care about the `PollProgress` from old sockets.
            todo!()
            /*  let poll_res =
                self.recv_state
                    .poll_socket(cx, &mut self.inner, &**socket, &*self.runtime, now);
            if poll_res.is_err() {
                self.prev_socket = None;
            } */
        };
        let poll_res =
            self.recv_state
                .poll_socket(cx, &mut self.inner, &*self.socket, &*self.runtime, now);

        self.recv_state.recv_limiter.finish_cycle(get_time);
        let poll_res = poll_res?;
        if poll_res.received_connection_packet {
            // Traffic has arrived on self.socket, therefore there is no need for the abandoned
            // one anymore. TODO: Account for multiple outgoing connections.
            todo!() // self.prev_socket = None;
        }
        Ok(poll_res.keep_going)
    }
    /// 2.
    fn handle_events(&mut self, cx: &mut Context, shared: &Shared) -> bool {
        for _ in 0..IO_LOOP_BOUND {
            let (ch, event): (ConnectionHandle, EndpointEvent) = match self.events.poll_recv(cx) {
                Poll::Ready(Some(x)) => {
                    todo!() // x,
                }
                Poll::Ready(None) => unreachable!("EndpointInner owns one sender"),
                Poll::Pending => {
                    return false;
                }
            };
            todo!()
        }
        todo!()
    }
}

/// A future that drives IO on an endpoint
///
/// This task functions as the switch point between the UDP socket object and the
/// `Endpoint` responsible for routing datagrams to their owning `Connection`.
/// In order to do so, it also facilitates the exchange of different types of events
/// flowing between the `Endpoint` and the tasks managing `Connection`s. As such,
/// running this task is necessary to keep the endpoint's connections running.
///
/// `EndpointDriver` futures terminate when all clones of the `Endpoint` have been dropped, or when
/// an I/O error occurs.
#[must_use = "endpoint drivers must be spawned for I/O to occur"]
#[derive(Debug)]
pub(crate) struct EndpointDriver(pub(crate) EndpointRef);

impl Future for EndpointDriver {
    type Output = Result<(), io::Error>;

    #[allow(unused_mut)] // MSRV
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut endpoint = self.0.state.lock().unwrap();
        if endpoint.driver.is_none() {
            endpoint.driver = Some(cx.waker().clone());
        }

        let now = endpoint.runtime.now();
        let mut keep_going = false;

        keep_going |= endpoint.drive_recv(cx, now)?;
        keep_going |= endpoint.handle_events(cx, &self.0.shared);

        if !endpoint.recv_state.incoming.is_empty() {
            self.0.shared.incoming.notify_waiters();
        }

        if endpoint.ref_count == 0 && endpoint.recv_state.connections.is_empty() {
            todo!() //    Poll::Ready(Ok(()))
        } else {
            drop(endpoint);
            // If there is more work to do schedule the endpoint task again.
            // `wake_by_ref()` is called outside the lock to minimize
            // lock contention on a multithreaded runtime.
            if keep_going {
                todo!() // cx.waker().wake_by_ref();
            }
            Poll::Pending
        }
    }
}

impl Drop for EndpointDriver {
    fn drop(&mut self) {
        let mut endpoint = self.0.state.lock().unwrap();
        endpoint.driver_lost = true;
        self.0.shared.incoming.notify_waiters();
        // Drop all outgoing channels, signaling the termination of the endpoint to the associated
        // connections.
        endpoint.recv_state.connections.senders.clear();
    }
}

#[derive(Debug)]
struct ConnectionSet {
    /// 1. Set if the endpoint has been manually closed
    close: Option<(VarInt, Bytes)>,
    /// 2. Senders for communicating with the endpoint's connections
    senders: FxHashMap<ConnectionHandle, mpsc::UnboundedSender<ConnectionEvent>>,
    /// 3. Stored to give out clones to new ConnectionInners
    sender: mpsc::UnboundedSender<(ConnectionHandle, EndpointEvent)>,
}

impl ConnectionSet {
    fn is_empty(&self) -> bool {
        self.senders.is_empty()
    }
    fn insert(
        &mut self,
        handle: ConnectionHandle,
        conn: proto::Connection,
        socket: Arc<dyn AsyncUdpSocket>,
        runtime: Arc<dyn Runtime>,
    ) -> Connecting {
        let (send, recv) = mpsc::unbounded_channel();
        if let Some((error_code, ref reason)) = self.close {
            todo!()
        }
        self.senders.insert(handle, send);
        Connecting::new(handle, conn, self.sender.clone(), recv, socket, runtime)
    }
}

#[derive(Default)]
struct PollProgress {
    /// Whether a datagram was routed to an existing connection
    received_connection_packet: bool,
    /// Whether datagram handling was interrupted early by the work limiter for fairness
    keep_going: bool,
}

/// Statistics on [Endpoint] activity
#[non_exhaustive]
#[derive(Debug, Default, Copy, Clone)]
pub struct EndpointStats {
    /// Cummulative number of Quic handshakees sent from this [Endpoint]
    pub outgoing_handshakes: u64,
}
