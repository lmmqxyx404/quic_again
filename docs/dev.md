# solve
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/connection/paths.rs:176:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::paths::RttEstimator::update
             at ./src/connection/paths.rs:176:13
   4: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2629:13
   5: scratch_quinn_proto::connection::Connection::process_early_payload
             at ./src/connection/mod.rs:1132:21
   6: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:822:17
   7: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:599:21
   8: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:435:13
   9: scratch_quinn_proto::connection::Connection::handle_coalesced
             at ./src/connection/mod.rs:457:21
  10: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:394:21
  11: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:466:25
  12: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
  13: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:286:9
  14: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:206:9
  15: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  16: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:180:9
  17: scratch_quinn_proto::tests::util::Pair::connect
             at ./src/tests/util.rs:171:9
  18: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:185:26
  19: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  20: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  21: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```