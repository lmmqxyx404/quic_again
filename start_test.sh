# RUSTFLAGS="-Awarnings" cargo watch -x "test "
clear
# RUST_BACKTRACE=full 
# RUST_BACKTRACE=1 
# RUSTFLAGS="-Awarnings" 
# cargo watch -x "test tests::server_stateless_reset"
RUST_BACKTRACE=1  RUSTFLAGS="-Awarnings"  cargo watch -x "test tests::key_update_simple"
# tests::server_stateless_reset