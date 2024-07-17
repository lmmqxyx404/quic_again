# quic_again
Learn the implementation of quic

# basic knowledge
## Frame
RFC 12.4

## Packet
RFC 12.1
初始包

握手包

受保护的包

0rtt包

一个packet 就是一个 datagram

## stream
有序子节流

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
