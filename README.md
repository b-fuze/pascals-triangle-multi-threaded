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

#### Running without any printing
Run the following to disable any code that involves printing
```sh
cargo run --release 10 --features=no-output
```

### Compiling
```sh
cargo build --release
```
It should be located at `target/release/pascals-triangle-rust{.exe,}`

#### Compiling without any printing
```sh
cargo build --release --features=no-output
```
The binary will be in the same place

To use it to calculate 10 rows for instance:
```sh
pascals-triangle-rust 10
```

[crossbeam-channel]: https://docs.rs/crossbeam/latest/crossbeam/channel/index.html

### License
MIT