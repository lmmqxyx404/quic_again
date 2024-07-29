# RUSTFLAGS="-Awarnings" cargo watch -x "test "
clear
RUSTFLAGS="-Awarnings" cargo watch -x "test tests::server_stateless_reset"
# tests::server_stateless_reset