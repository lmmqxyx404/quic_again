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

# `impl fmt::Debug for Ack` pay attention to `Write`
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/frame.rs:657:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: <scratch_quinn_proto::frame::Ack as core::fmt::Debug>::fmt
             at ./src/frame.rs:657:9
   4: <&T as core::fmt::Debug>::fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:2343:62
   5: core::fmt::builders::DebugTuple::field::{{closure}}
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/builders.rs:330:29
   6: core::fmt::builders::DebugTuple::field_with::{{closure}}
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/builders.rs:355:17
   7: core::result::Result<T,E>::and_then
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/result.rs:1321:22
   8: core::fmt::builders::DebugTuple::field_with
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/builders.rs:342:35
   9: core::fmt::builders::DebugTuple::field
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/builders.rs:330:9
  10: core::fmt::Formatter::debug_tuple_field1_finish
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:2105:9
  11: <scratch_quinn_proto::frame::Frame as core::fmt::Debug>::fmt
             at ./src/frame.rs:279:10
  12: <&T as core::fmt::Debug>::fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:2343:62
  13: core::fmt::rt::Argument::fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/rt.rs:165:63
  14: core::fmt::write
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:1157:21
  15: <&T as core::fmt::Debug>::fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:2343:62
  16: core::fmt::rt::Argument::fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/rt.rs:165:63
  17: core::fmt::write
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:1157:21
  18: <&mut W as core::fmt::Write::write_fmt::SpecWriteFmt>::spec_write_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:218:21
  19: core::fmt::Write::write_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:223:9
  20: <&mut W as core::fmt::Write>::write_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/fmt/mod.rs:238:9
  21: tracing_subscriber::fmt::format::Writer::write_fmt
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/format/mod.rs:514:9
  22: <tracing_subscriber::fmt::format::Writer as core::fmt::Write>::write_fmt
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/format/mod.rs:574:9
  23: tracing_subscriber::fmt::format::Writer::write_fmt
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/format/mod.rs:514:9
  24: <tracing_subscriber::fmt::format::DefaultVisitor as tracing_core::field::Visit>::record_debug
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/format/mod.rs:1279:26
  25: <core::fmt::Arguments as tracing_core::field::Value>::record
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/field.rs:615:9
  26: <&T as tracing_core::field::Value>::record
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/field.rs:594:9
  27: tracing_core::field::ValueSet::record
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/field.rs:1006:17
  28: tracing_core::event::Event::record
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/event.rs:87:9
  29: <tracing_core::event::Event as tracing_subscriber::field::RecordFields>::record
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/field/mod.rs:163:9
  30: <&F as tracing_subscriber::field::RecordFields>::record
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/field/mod.rs:187:9
  31: <M as tracing_subscriber::fmt::format::FormatFields>::format_fields
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/format/mod.rs:1166:9
  32: <tracing_subscriber::fmt::fmt_layer::FmtContext<S,N> as tracing_subscriber::fmt::format::FormatFields>::format_fields
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/fmt_layer.rs:1033:9
  33: <tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Full,T> as tracing_subscriber::fmt::format::FormatEvent<S,N>>::format_event
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/format/mod.rs:1021:9
  34: <tracing_subscriber::fmt::fmt_layer::Layer<S,N,E,W> as tracing_subscriber::layer::Layer<S>>::on_event::{{closure}}
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/fmt_layer.rs:965:16
  35: std::thread::local::LocalKey<T>::try_with
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/thread/local.rs:286:12
  36: std::thread::local::LocalKey<T>::with
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/thread/local.rs:262:9
  37: <tracing_subscriber::fmt::fmt_layer::Layer<S,N,E,W> as tracing_subscriber::layer::Layer<S>>::on_event
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/fmt_layer.rs:949:9
  38: <tracing_subscriber::layer::layered::Layered<L,S> as tracing_core::subscriber::Subscriber>::event
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/layer/layered.rs:153:9
  39: <tracing_subscriber::layer::layered::Layered<L,S> as tracing_core::subscriber::Subscriber>::event
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/layer/layered.rs:152:9
  40: <tracing_subscriber::fmt::Subscriber<N,E,F,W> as tracing_core::subscriber::Subscriber>::event
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-subscriber-0.3.18/src/fmt/mod.rs:408:9
  41: tracing_core::dispatcher::Dispatch::event
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/dispatcher.rs:615:13
  42: tracing_core::event::Event::dispatch::{{closure}}
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/event.rs:35:13
  43: tracing_core::dispatcher::get_default::{{closure}}
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/dispatcher.rs:397:24
  44: std::thread::local::LocalKey<T>::try_with
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/thread/local.rs:286:12
  45: tracing_core::dispatcher::get_default
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/dispatcher.rs:394:5
  46: tracing_core::event::Event::dispatch
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-core-0.1.32/src/event.rs:34:9
  47: scratch_quinn_proto::connection::Connection::process_payload::{{closure}}
             at /home/si/.cargo/registry/src/mirrors.sjtug.sjtu.edu.cn-7a04d2510079875b/tracing-0.1.40/src/macros.rs:875:17
  48: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:965:21
  49: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:704:38
  50: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:599:21
  51: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:435:13
  52: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:385:17
  53: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:466:25
  54: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
  55: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:286:9
  56: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:206:9
  57: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  58: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:180:9
  59: scratch_quinn_proto::tests::util::Pair::connect
             at ./src/tests/util.rs:171:9
  60: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:185:26
  61: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  62: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  63: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/connection/mtud.rs:81:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::mtud::MtuDiscovery::on_acked
             at ./src/connection/mtud.rs:81:9
   4: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2601:35
   5: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1000:21
   6: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:704:38
   7: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:599:21
   8: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:435:13
   9: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:385:17
  10: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:466:25
  11: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
  12: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:286:9
  13: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:206:9
  14: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  15: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:180:9
  16: scratch_quinn_proto::tests::util::Pair::connect
             at ./src/tests/util.rs:171:9
  17: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:185:26
  18: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  20: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

# 
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/connection/mtud.rs:96:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::mtud::MtuDiscovery::on_acked
             at ./src/connection/mtud.rs:96:13
   4: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2601:35
   5: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1000:21
   6: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:704:38
   7: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:599:21
   8: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:435:13
   9: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:385:17
  10: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:466:25
  11: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
  12: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:286:9
  13: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:206:9
  14: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  15: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:180:9
  16: scratch_quinn_proto::tests::util::Pair::connect
             at ./src/tests/util.rs:171:9
  17: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:185:26
  18: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  20: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

# pay attention to `145`
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/connection/mod.rs:1614:13:
assertion failed: buf_capacity - buf.len() >= MIN_PACKET_SPACE
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::Connection::poll_transmit
             at ./src/connection/mod.rs:1614:13
   4: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:474:44
   5: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
   6: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:256:9
   7: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:205:9
   8: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
   9: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:193:5
  10: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  11: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  12: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/connection/mtud.rs:96:13:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::mtud::MtuDiscovery::on_acked
             at ./src/connection/mtud.rs:96:13
   4: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2636:35
   5: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1000:21
   6: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:704:38
   7: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:599:21
   8: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:435:13
   9: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:385:17
  10: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:466:25
  11: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
  12: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:256:9
  13: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:205:9
  14: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  15: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:180:9
  16: scratch_quinn_proto::tests::util::Pair::connect
             at ./src/tests/util.rs:171:9
  17: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:185:26
  18: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  20: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/connection/mod.rs:2638:21:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::Connection::on_ack_received
             at ./src/connection/mod.rs:2638:21
   4: scratch_quinn_proto::connection::Connection::process_payload
             at ./src/connection/mod.rs:1000:21
   5: scratch_quinn_proto::connection::Connection::process_decrypted_packet
             at ./src/connection/mod.rs:704:38
   6: scratch_quinn_proto::connection::Connection::handle_packet
             at ./src/connection/mod.rs:599:21
   7: scratch_quinn_proto::connection::Connection::handle_decode
             at ./src/connection/mod.rs:435:13
   8: scratch_quinn_proto::connection::Connection::handle_event
             at ./src/connection/mod.rs:385:17
   9: scratch_quinn_proto::tests::util::TestEndpoint::drive_outgoing
             at ./src/tests/util.rs:466:25
  10: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:398:9
  11: scratch_quinn_proto::tests::util::Pair::drive_client
             at ./src/tests/util.rs:256:9
  12: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:205:9
  13: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
  14: scratch_quinn_proto::tests::util::Pair::connect_with
             at ./src/tests/util.rs:180:9
  15: scratch_quinn_proto::tests::util::Pair::connect
             at ./src/tests/util.rs:171:9
  16: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:185:26
  17: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  18: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  19: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```
#
```
thread 'tests::server_stateless_reset' panicked at quinn-proto/src/tests/mod.rs:194:5:
assertion failed: `Some(ConnectionLost { reason: ConnectionClosed(ConnectionClose { reason: b"invalid frame ID", error_code: FRAME_ENCODING_ERROR, frame_type: Some(PING) }) })` does not match `Some(Event::ConnectionLost { reason: ConnectionError::Reset })`
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:194:5
   3: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
   4: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   5: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::tests::util::TestEndpoint::drive_incoming
             at ./src/tests/util.rs:445:25
   4: scratch_quinn_proto::tests::util::TestEndpoint::drive
             at ./src/tests/util.rs:397:9
   5: scratch_quinn_proto::tests::util::Pair::drive_server
             at ./src/tests/util.rs:286:9
   6: scratch_quinn_proto::tests::util::Pair::step
             at ./src/tests/util.rs:206:9
   7: scratch_quinn_proto::tests::util::Pair::drive
             at ./src/tests/util.rs:201:15
   8: scratch_quinn_proto::tests::server_stateless_reset
             at ./src/tests/mod.rs:193:5
   9: scratch_quinn_proto::tests::server_stateless_reset::{{closure}}
             at ./src/tests/mod.rs:172:28
  10: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  11: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

# 
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/tests/mod.rs:311:59:
called `Option::unwrap()` on a `None` value
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: core::option::unwrap_failed
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/option.rs:1984:5
   4: core::option::Option<T>::unwrap
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/option.rs:932:21
   5: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:311:13
   6: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/connection/streams/mod.rs:152:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::streams::SendStream::write_source
             at ./src/connection/streams/mod.rs:152:9
   4: scratch_quinn_proto::connection::streams::SendStream::write
             at ./src/connection/streams/mod.rs:140:12
   5: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:314:5
   6: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/connection/streams/send.rs:164:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: <scratch_quinn_proto::connection::streams::send::ByteSlice as scratch_quinn_proto::connection::streams::send::BytesSource>::pop_chunk
             at ./src/connection/streams/send.rs:164:9
   4: scratch_quinn_proto::connection::streams::send::Send::write
             at ./src/connection/streams/send.rs:74:44
   5: scratch_quinn_proto::connection::streams::SendStream::write_source
             at ./src/connection/streams/mod.rs:185:23
   6: scratch_quinn_proto::connection::streams::SendStream::write
             at ./src/connection/streams/mod.rs:146:12
   7: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:314:5
   8: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
   9: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
  10: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```

#
```
thread 'tests::finish_stream_simple' panicked at quinn-proto/src/connection/streams/mod.rs:154:9:
not yet implemented
stack backtrace:
   0: rust_begin_unwind
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/panicking.rs:652:5
   1: core::panicking::panic_fmt
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:72:14
   2: core::panicking::panic
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/panicking.rs:146:5
   3: scratch_quinn_proto::connection::streams::SendStream::finish
             at ./src/connection/streams/mod.rs:154:9
   4: scratch_quinn_proto::tests::finish_stream_simple
             at ./src/tests/mod.rs:316:5
   5: scratch_quinn_proto::tests::finish_stream_simple::{{closure}}
             at ./src/tests/mod.rs:306:26
   6: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/129f3b9964af4d4a709d1383930ade12dfe7c081/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
```