# io-uring-bench-rust

Rust benchmark playground for comparing ordinary file copy with future io_uring versions.

## Run

```bash
source "$HOME/.cargo/env"
cargo run --release --bin rw_copy -- test.bin out.bin
```
