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

# dev skill
## Do not omit `Drop`
