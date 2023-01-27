# Pascal's Triangle Multi-threaded

Uses [crossbeam-channel] for cross-thread communication. The code
is very bad and messy.

## Usage

### Running
Get a Rust compiler and run
```sh
cargo run --release 10
```

The number is the number of rows to calculate

### Compiling
```sh
cargo build --release
```
It should be located at `target/release/pascals-triangle-rust{.exe,}`

To use it to calculate 10 rows for instance:
```sh
pascals-triangle-rust 10
```

[crossbeam-channel]: https://docs.rs/crossbeam/latest/crossbeam/channel/index.html

### License
MIT