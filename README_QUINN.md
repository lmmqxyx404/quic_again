# dev history
## 1. about `rustls` crate in `Cargo.toml`
注意引入方式与使用方式，在`tests::RootCertStore`

### 注意`features.rustls`之后添加的依赖项

## 2. `connection.rs` 之中的 `Connection`
这个模块中会自行定义一个`struct Connection`

## `impl Drop for EndpointRef`
When call `state.lock().unwrap();`, It may cause error.
All you need to do is to impl other tokio task. So you could get the lock.

## 3.`impl Endpoint::connect_with`
### `endpoint::State::drive_recv`
add `recv_limiter`

* `State::drive_recv -> RecvState::poll_socket`
* * first time : ended with Poll::Pending, Then drive_recv ended.

* `handle_events` -> `self.events.poll_recv(cx)`
* * pay attention to the `events::mpsc::UnboundedReceiver<(ConnectionHandle, EndpointEvent)>`

* * 应该要注意对应的 `mpsc::UnboundedSender`
如果缺少了对应的 `sender` ，`receiver` 就无法正常的 call `poll_recv`.
正常call 应该初始是 `Pending`, 异常就是 `Ready(None)`

## 4. Endpoint::clinet
`Self::new_with_abstract_socket` -> `driver.await` -> `endpoint.drive_recv` -> `endpoint.handle_events`

## 5.`Endpoint::connect_with`
* `connections.insert`[ConnectionSet::insert] -> `Connecting::new`
-> `Clone for ConnectionRef`

## 6. `Connecting::poll` called in `handshake_timeout`
`client.connect_with().unwrap.await` -> `Pin::new(&mut self.connected).poll(cx)`
注意后面调用这个poll 方法时，要将与之关联的 `sender` 正确设置.
也就是 `connection::State::on_connected`

## 7. `5 -> Future for ConnectionDriver`
`poll conn.drive_transmit` -> `State::drive_transmit` -> `self.socket.try_send` ->
`impl AsyncUdpSocket for UdpSocket ::try_send` -> `self.inner.send` -> `UdpSocketState::send` -> `unix::send` -> `prepare_msg`

* `Future for ConnectionDriver`
* * `conn.drive_transmit`
* * `conn.drive_timer`
* * `conn.forward_endpoint_events`
* * `conn.forward_app_events`

## 8. about `tests::close_endpoint`
`match conn.await` -> `process_conn_events` -> `drive_transmit`

## 9. about `tests::read_after_close`
`endpoint2.accept().await.expect("endpoint").await.expect("connection");` -> `Endpoint::Accept` ->
`Future for Accept` -> `IntoFuture for Incoming` -> `IncomingFuture(self.accept())`
-> `Incoming::accept` -> `state.endpoint.accept` -> `EndpointInner::accept`

* second step
`Future for ConnectionDriver` -> `endpoint.drive_recv` -> `self.recv_state.poll_socket` ->
`match socket.poll_recv` -> `AsyncUdpSocket for UdpSocket::poll_recv`
-> `self.inner.recv` -> `UdpSocketState::recv` -> `decode_recv`

* seconde step finished. Then
`socket.poll_recv` -> `Poll::Ready(Ok(msgs))` -> `match endpoint.handle()` -> `Some(DatagramEvent::NewConnection(incoming))`

# dev skill
## Do not omit `Drop`

## watch the tokio relative paired Api, like `Sender` `Receiver` carefully.