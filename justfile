test_all:
  cargo build --release
  cargo test --release
  cargo nextest run --release

format_all:
  cargo +nightly fmt
