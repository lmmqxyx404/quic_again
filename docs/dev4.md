# `idle_timeout`
```
thread 'tests::idle_timeout' panicked at quinn-proto/src/connection/mod.rs:1424:21:
internal error: entered unreachable code:  handle_timeout Idle
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: scratch_quinn_proto::connection::Connection::handle_timeout
             at ./src/connection/mod.rs:1424:21
   3: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:494:21
   4: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:426:9
   5: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:290:9
   6: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
   7: scratch_quinn_proto::tests::idle_timeout
             at ./src/tests/mod.rs:1141:13
   8: scratch_quinn_proto::tests::idle_timeout::{{closure}}
             at ./src/tests/mod.rs:1123:18
   9: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  10: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

