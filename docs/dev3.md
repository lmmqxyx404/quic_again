#
```
thread 'tests::key_update_simple' panicked at quinn-proto/src/tests/mod.rs:978:10:
couldn't open first stream
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic_display
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:263:5
   3: core::option::expect_failed
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/option.rs:1994:5
   4: core::option::Option<T>::expect
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/option.rs:895:21
   5: scratch_quinn_proto::tests::key_update_simple
             at ./src/tests/mod.rs:971:13
   6: scratch_quinn_proto::tests::key_update_simple::{{closure}}
             at ./src/tests/mod.rs:967:23
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::key_update_simple' panicked at quinn-proto/src/tests/mod.rs:981:48:
called `Result::unwrap()` on an `Err` value: Blocked
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::result::unwrap_failed
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/result.rs:1654:5
   3: core::result::Result<T,E>::unwrap
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/result.rs:1077:23
   4: scratch_quinn_proto::tests::key_update_simple
             at ./src/tests/mod.rs:981:5
   5: scratch_quinn_proto::tests::key_update_simple::{{closure}}
             at ./src/tests/mod.rs:967:23
   6: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::key_update_simple' panicked at quinn-proto/src/connection/mod.rs:2064:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::Connection::update_keys
             at ./src/connection/mod.rs:2064:9
   4: scratch_quinn_proto::connection::Connection::initiate_key_update
             at ./src/connection/mod.rs:2060:9
   5: scratch_quinn_proto::tests::key_update_simple
             at ./src/tests/mod.rs:999:5
   6: scratch_quinn_proto::tests::key_update_simple::{{closure}}
             at ./src/tests/mod.rs:967:23
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::key_update_simple' panicked at quinn-proto/src/connection/assembler.rs:51:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::assembler::Assembler::insert
             at ./src/connection/assembler.rs:51:13
   4: scratch_quinn_proto::connection::streams::recv::Recv::ingest
             at ./src/connection/streams/recv.rs:104:13
   5: scratch_quinn_proto::connection::streams::state::StreamsState::received
             at ./src/connection/streams/state.rs:469:13
   6: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1035:24
   7: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:725:38
   8: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:608:21
   9: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:444:13
  10: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:394:17
  11: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:499:25
  12: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:426:9
  13: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:290:9
  14: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
  15: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:205:15
  16: scratch_quinn_proto::tests::key_update_simple
             at ./src/tests/mod.rs:1003:5
  17: scratch_quinn_proto::tests::key_update_simple::{{closure}}
             at ./src/tests/mod.rs:967:23
  18: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```