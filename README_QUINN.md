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

* `State::drive_recv -> RecvState::poll_socket `