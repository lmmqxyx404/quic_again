# quic_again
Learn the implementation of quic

# basic knowledge
## Frame
RFC 12.4

### `tests::migration`
add 3 main Frame
`Frame::PathResponse`
`Frame::PATH_CHALLENGE`
`Frame::IMMEDIATE_ACK`

## Packet
RFC 12.1
初始包

握手包

受保护的包

0rtt包

一个packet 就是一个 datagram

## stream
有序子节流

### process
`let mut recv = pair.server_recv(server_ch, s);`
收到一个 RecvStream
`let mut chunks = recv.read(false).unwrap();`
再读取 stream ，得到 Chunks
Chunks 这个数据结构有`StreamId`，用以区分不同的数据
`Chonks::new` -> `ensure_ordering`
`self.defragment();`
`recvd.insert(0..self.bytes_read);`


## connection
可以进行迁移


## real rust code
### 1. PacketNumber (atom)
- fn encode() completed

### 2. Packet(combined from 3)

### 3. Header(atom)

connectionId

# todo
## need to optimize
TransportParameters

## about `State::Closed(_)`
有许多细节需要去查看

## `StreamsState::insert`
在创建时，就需要调用这个方法

## `pub(super) struct Recv` vital

## `Connection::mod`
Err
detect_lost
search todo

# dev skills
## 1. about transport_error::Error
实现 `Display` 是为了实现 `std::error::Error`
而 `std::error::Error ` 是为了 `ConnectionError::TransportError`
而 `ConnectionError::TransportError` 是为了 `Err(e.into())`
注意其中的联系

## 2. about `[dev-dependencies]` in `Cargo.toml`
```Rust
#[cfg(all(test, feature = "rustls"))]
mod tests;
```
以上代码如果缺少相关 cfg 设置，那么就会报错，无法识别`[dev-dependencies]` 配置的 crate

## 3. about `Index trait`
```Rust
use std::ops::{Index, IndexMut};
impl Index<SpaceId> for [PacketSpace; 3]
impl IndexMut<SpaceId> for [PacketSpace; 3]
```
这个trait可以使用别的 struct 当作索引

## 4. about `ConnectionClose`
这个结构体的实现比较复杂
主要是其中的一个field `pub error_code: TransportErrorCode,`

## 5. about trait default implementation.
`Controller::on_sent`
这个成员就是一个默认实现

## 6. 扩展第三方struct，并为其实现新的trait
那么在使用的时候，注意既要引入第三方 crate, 也需要引入自定义的 trait
具体代码示例请看,
```Rust
let id = r.get_var()?;
let len = r.get_var()?;
```
以上代码的错误处理也需要关注

# dev history
## 1. 在 `4c8ab712de949a` 之后一个 commit
就让 `fn version_negotiate_client` 通过test了，注意之后的 `assert_matches!`.
第一遍的时候搞错了

## 2. about Connection::new
在创建 Connection 时，如果 `side.is_client()`, 那么记得
```Rust
this.write_crypto();
this.init_0rtt();
```
## 在 `4c8ab712de949a` 之后一个 commit
就让 `fn version_negotiate_client` 通过test了

## 3. about `try_next`
解析数据时，要知道quic 协议连接的细节

## 4. about `TransportParameters`
pay attention to the new fn.
wrong parameters will result in different error.
### `initial_src_cid: Some(initial_src_cid),`
The above is related with `Connection::handle_peer_params`

## 5. about `std::ops::Not` trait
impl the trait so that to support bool operation

## 6.pub trait AeadKey
这个 `trait` 用来加密和解密数据
`seal` `open`

# quic connection details
无状态连接，与有状态连接
## first packet
注意第一个包的发送与接收
## 2. stateless connection

# debug history
## `if let Header::Initial(InitialHeader { ref token, .. }) = packet.header`
缺少了一段代码逻辑

## `pub(super) fn is_empty(&self, streams: &StreamsState) -> bool {`
完善逻辑

## `_span: span,`
需要补充逻辑

## The following is the right impl
```
impl Default for MtuDiscoveryConfig {
    fn default() -> Self {
        Self {
            upper_bound: 1452,
            interval: Duration::from_secs(600),
            minimum_change: 20,
            black_hole_cooldown: Duration::from_secs(60),

        }
    }
}
```

## donot omit the following logic
```
// PING
        if mem::replace(&mut space.ping_pending, false) {
            trace!("PING");
            buf.write(frame::Type::PING);
            sent.non_retransmits = true;
            self.stats.frame_tx.ping += 1;
        }
```