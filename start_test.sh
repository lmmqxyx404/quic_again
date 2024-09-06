# RUSTFLAGS="-Awarnings" cargo watch -x "test "
clear
# RUST_BACKTRACE=full 
# RUST_BACKTRACE=1 
# RUSTFLAGS="-Awarnings" 
# cargo watch -x "test tests::server_stateless_reset"
# RUST_BACKTRACE=1  RUSTFLAGS="-Awarnings"  
# cargo watch -x "test -p scratch-quinn tests::handshake_timeout"

#RUST_BACKTRACE=1  RUSTFLAGS="-Awarnings"  
cargo watch -x "test -p scratch-quinn --lib -- tests::close_endpoint"
# tests::server_stateless_reset