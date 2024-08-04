# migration
```
thread 'tests::migration' panicked at quinn-proto/src/connection/mod.rs:1159:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1159:13
   4: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:725:38
   5: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:608:21
   6: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:444:13
   7: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:394:17
   8: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:499:25
   9: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:426:9
  10: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:290:9
  11: scratch_quinn_proto::tests::migration
             at ./src/tests/mod.rs:1225:5
  12: scratch_quinn_proto::tests::migration::{{closure}}
             at ./src/tests/mod.rs:1208:15
  13: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  14: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::migration' panicked at quinn-proto/src/connection/mod.rs:1480:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:1480:13
   4: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:507:44
   5: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:426:9
   6: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:290:9
   7: scratch_quinn_proto::tests::migration
             at ./src/tests/mod.rs:1225:5
   8: scratch_quinn_proto::tests::migration::{{closure}}
             at ./src/tests/mod.rs:1208:15
   9: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  10: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

# to solve the following error.I should add a new `Frame` type
```
thread 'tests::migration' panicked at quinn-proto/src/tests/mod.rs:1229:5:
assertion failed: `Some(ConnectionLost { reason: TransportError(Error { code: FRAME_ENCODING_ERROR, frame: Some(PATH_CHALLENGE), reason: "invalid frame ID" }) })` does not match `None`
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: scratch_quinn_proto::tests::migration
             at ./src/tests/mod.rs:1229:5
   3: scratch_quinn_proto::tests::migration::{{closure}}
             at ./src/tests/mod.rs:1208:15
   4: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   5: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```