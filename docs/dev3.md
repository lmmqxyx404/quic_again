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