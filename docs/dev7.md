
# `datagram_recv_buffer_overflow`
```
thread 'tests::datagram_recv_buffer_overflow' panicked at quinn-proto/src/connection/datagrams.rs:73:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::datagrams::DatagramState::received
             at ./src/connection/datagrams.rs:73:13
   4: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1058:24
   5: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:732:38
   6: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:615:21
   7: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:451:13
   8: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:401:17
   9: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:512:25
  10: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:439:9
  11: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:291:9
  12: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:211:9
  13: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  14: scratch_quinn_proto::tests::datagram_recv_buffer_overflow
             at ./src/tests/mod.rs:1789:5
  15: scratch_quinn_proto::tests::datagram_recv_buffer_overflow::{{closure}}
             at ./src/tests/mod.rs:1759:35
  16: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  17: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```