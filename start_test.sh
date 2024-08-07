# RUSTFLAGS="-Awarnings" cargo watch -x "test "
clear
# RUST_BACKTRACE=full 
# RUST_BACKTRACE=1 
# RUSTFLAGS="-Awarnings" 
# cargo watch -x "test tests::server_stateless_reset"
# RUST_BACKTRACE=1  RUSTFLAGS="-Awarnings"  
cargo watch -x "test -p scratch-quinn-proto tests::migrate_detects_new_mtu_and_respects_original_peer_max_udp_payload_size"
# tests::server_stateless_reset