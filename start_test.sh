# RUSTFLAGS="-Awarnings" cargo watch -x "test "
clear
# RUST_BACKTRACE=full 
# RUST_BACKTRACE=1 
# RUSTFLAGS="-Awarnings" 
# cargo watch -x "test tests::server_stateless_reset"
# RUST_BACKTRACE=1  RUSTFLAGS="-Awarnings" 
cargo watch -x "test tests::alpn_success"
# tests::server_stateless_reset