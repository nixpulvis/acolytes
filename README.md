# Session Types

Implementation of "Session Types for Rust" - (Munksgaard, et. all)

### Oblivious Transfer (OT) Example

In this example, the sender has two values in it's memory and the receiver can
pick either the first or second value by index. The receiver only learns the
value it chose and the sender doesn't know which value it revieled.

```sh
# Start the receiving client first, this will bind to the socket.
cargo run --example ot -- --receiver
# Then start the sending client, which will connect to the receiver.
cargo run --example ot -- --sender
```

To see a trace of the program execution, run the examples with
`RUST_LOG=debug`.

### Testing

```sh
# Test the session-types crate
cargo test -- --test-threads=1
# Test the underlying channels crate
cargo test -p channels -- --test-threads=1
```
