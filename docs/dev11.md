#
```
thread 'tests::ack_frequency_ack_delayed_from_first_of_flight' panicked at quinn-proto/src/connection/ack_frequency.rs:46:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::ack_frequency::AckFrequencyState::should_send_ack_frequency
             at ./src/connection/ack_frequency.rs:46:9
   4: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:1650:64
   5: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:545:44
   6: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:464:9
   7: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:261:9
   8: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
   9: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  10: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:185:9
  11: scratch_quinn_proto::tests::setup_ack_frequency_test
             at ./src/tests/mod.rs:2634:34
  12: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight
             at ./src/tests/mod.rs:2652:44
  13: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight::{{closure}}
             at ./src/tests/mod.rs:2650:52
  14: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  15: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::ack_frequency_ack_delayed_from_first_of_flight' panicked at quinn-proto/src/connection/ack_frequency.rs:58:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::ack_frequency::AckFrequencyState::next_sequence_number
             at ./src/connection/ack_frequency.rs:58:9
   4: scratch_quinn_proto::connection::Connection::populate_packet
             at ./src/connection/mod.rs:2360:35
   5: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:2003:17
   6: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:545:44
   7: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:464:9
   8: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:261:9
   9: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
  10: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  11: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:185:9
  12: scratch_quinn_proto::tests::setup_ack_frequency_test
             at ./src/tests/mod.rs:2634:34
  13: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight
             at ./src/tests/mod.rs:2652:44
  14: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight::{{closure}}
             at ./src/tests/mod.rs:2650:52
  15: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  16: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```
#
```
thread 'tests::ack_frequency_ack_delayed_from_first_of_flight' panicked at quinn-proto/src/connection/ack_frequency.rs:72:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::ack_frequency::AckFrequencyState::candidate_max_ack_delay
             at ./src/connection/ack_frequency.rs:72:9
   4: scratch_quinn_proto::connection::Connection::populate_packet
             at ./src/connection/mod.rs:2366:33
   5: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:2003:17
   6: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:545:44
   7: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:464:9
   8: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:261:9
   9: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
  10: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  11: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:185:9
  12: scratch_quinn_proto::tests::setup_ack_frequency_test
             at ./src/tests/mod.rs:2634:34
  13: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight
             at ./src/tests/mod.rs:2652:44
  14: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight::{{closure}}
             at ./src/tests/mod.rs:2650:52
  15: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  16: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::ack_frequency_ack_delayed_from_first_of_flight' panicked at quinn-proto/src/connection/ack_frequency.rs:82:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::ack_frequency::AckFrequencyState::ack_frequency_sent
             at ./src/connection/ack_frequency.rs:82:9
   4: scratch_quinn_proto::connection::Connection::populate_packet
             at ./src/connection/mod.rs:2384:13
   5: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:2003:17
   6: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:545:44
   7: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:464:9
   8: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:261:9
   9: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
  10: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  11: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:185:9
  12: scratch_quinn_proto::tests::setup_ack_frequency_test
             at ./src/tests/mod.rs:2634:34
  13: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight
             at ./src/tests/mod.rs:2652:44
  14: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight::{{closure}}
             at ./src/tests/mod.rs:2650:52
  15: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  16: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::ack_frequency_ack_delayed_from_first_of_flight' panicked at quinn-proto/src/connection/ack_frequency.rs:54:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::ack_frequency::AckFrequencyState::should_send_ack_frequency
             at ./src/connection/ack_frequency.rs:54:9
   4: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:1650:64
   5: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:545:44
   6: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:464:9
   7: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:261:9
   8: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:210:9
   9: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  10: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:185:9
  11: scratch_quinn_proto::tests::setup_ack_frequency_test
             at ./src/tests/mod.rs:2634:34
  12: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight
             at ./src/tests/mod.rs:2652:44
  13: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight::{{closure}}
             at ./src/tests/mod.rs:2650:52
  14: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  15: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::ack_frequency_ack_delayed_from_first_of_flight' panicked at quinn-proto/src/frame.rs:235:25:
not implemented: current frame type is ACK_FREQUENCY
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: scratch_quinn_proto::frame::Iter::try_next
             at ./src/frame.rs:235:25
   3: <scratch_quinn_proto::frame::Iter as core::iter::traits::iterator::Iterator>::next
             at ./src/frame.rs:288:15
   4: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1000:23
   5: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:741:38
   6: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:624:21
   7: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:460:13
   8: scratch_quinn_proto::connection::Connection::handle_coalesced
             at ./src/connection/mod.rs:482:21
   9: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:419:21
  10: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:537:25
  11: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:464:9
  12: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:291:9
  13: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:211:9
  14: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:206:15
  15: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:185:9
  16: scratch_quinn_proto::tests::setup_ack_frequency_test
             at ./src/tests/mod.rs:2634:34
  17: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight
             at ./src/tests/mod.rs:2652:44
  18: scratch_quinn_proto::tests::ack_frequency_ack_delayed_from_first_of_flight::{{closure}}
             at ./src/tests/mod.rs:2650:52
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  20: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```