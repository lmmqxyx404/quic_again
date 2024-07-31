#
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/range_set/btree_range_set.rs:26:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::range_set::btree_range_set::RangeSet::insert
             at ./src/range_set/btree_range_set.rs:26:9
   4: scratch_quinn_proto::connection::send_buffer::SendBuffer::ack
             at ./src/connection/send_buffer.rs:130:9
   5: scratch_quinn_proto::connection::streams::send::Send::ack
             at ./src/connection/streams/send.rs:105:9
   6: scratch_quinn_proto::connection::streams::state::StreamsState::received_ack_of
             at ./src/connection/streams/state.rs:379:13
   7: scratch_quinn_proto::connection::Connection::on_packet_acked
             at ./src/connection/mod.rs:2727:13
   8: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2652:17
   9: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:999:21
  10: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:703:38
  11: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:598:21
  12: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:434:13
  13: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:384:17
  14: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:476:25
  15: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:406:9
  16: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:256:9
  17: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:205:9
  18: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  19: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:317:5
  20: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
  21: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  22: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/connection/streams/state.rs:353:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::streams::state::StreamsState::stream_freed
             at ./src/connection/streams/state.rs:353:9
   4: scratch_quinn_proto::connection::streams::state::StreamsState::received_ack_of
             at ./src/connection/streams/state.rs:385:9
   5: scratch_quinn_proto::connection::Connection::on_packet_acked
             at ./src/connection/mod.rs:2727:13
   6: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2652:17
   7: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:999:21
   8: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:703:38
   9: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:598:21
  10: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:434:13
  11: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:384:17
  12: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:476:25
  13: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:406:9
  14: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:256:9
  15: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:205:9
  16: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  17: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:317:5
  18: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  20: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

# about receive stream
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/connection/streams/mod.rs:262:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::streams::RecvStream::read
             at ./src/connection/streams/mod.rs:262:9
   4: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:335:22
   5: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
   6: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

# about `reset_stream`
add the `ResetStream` support
```
thread 'tests::reset_stream' panicked at quinn-proto/src/tests/mod.rs:371:5:
assertion failed: `Ok(Some(Chunk { offset: 0, bytes: b"hello" }))` does not match `Err(ReadError::Reset(ERROR))`
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: scratch_quinn_proto::tests::reset_stream
             at ./src/tests/mod.rs:371:5
   3: scratch_quinn_proto::tests::reset_stream::{{closure}}
             at ./src/tests/mod.rs:347:18
   4: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   5: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```