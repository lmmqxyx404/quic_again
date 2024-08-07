#
```
thread 'tests::single_ack_eliciting_packet_triggers_ack_after_delay' panicked at quinn-proto/src/tests/mod.rs:2463:5:
assertion `left == right` failed
  left: 1
 right: 0
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::assert_failed_inner
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:409:17
   3: core::panicking::assert_failed
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:364:5
   4: scratch_quinn_proto::tests::single_ack_eliciting_packet_triggers_ack_after_delay
             at ./src/tests/mod.rs:2463:5
   5: scratch_quinn_proto::tests::single_ack_eliciting_packet_triggers_ack_after_delay::{{closure}}
             at ./src/tests/mod.rs:2441:58
   6: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```