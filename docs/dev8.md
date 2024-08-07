# `migrate_detects_new_mtu_and_respects_original_peer_max_udp_payload_size`
```
thread 'tests::migrate_detects_new_mtu_and_respects_original_peer_max_udp_payload_size' panicked at quinn-proto/src/tests/mod.rs:2269:5:
assertion `left == right` failed
  left: 1293
 right: 1300
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::assert_failed_inner
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:409:17
   3: core::panicking::assert_failed
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:364:5
   4: scratch_quinn_proto::tests::migrate_detects_new_mtu_and_respects_original_peer_max_udp_payload_size
             at ./src/tests/mod.rs:2269:5
   5: scratch_quinn_proto::tests::migrate_detects_new_mtu_and_respects_original_peer_max_udp_payload_size::{{closure}}
             at ./src/tests/mod.rs:2241:77
   6: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```