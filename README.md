# flume-overwrite

[![Crates.io](https://img.shields.io/crates/v/flume-overwrite.svg)](https://crates.io/crates/flume-overwrite)
[![Documentation](https://docs.rs/flume-overwrite/badge.svg)](https://docs.rs/flume-overwrite)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A Rust library that provides bounded channels with overwrite capability, built on top of the [flume](https://github.com/zesterer/flume) crate. When the channel reaches capacity, new messages will automatically overwrite the oldest unread messages, ensuring that sends never block due to a full channel.

## Features

- **Overwrite semantics**: Messages sent to a full channel replace the oldest messages
- **Non-blocking sends**: Never blocks when sending, even at capacity  
- **Async support**: Both blocking and async send operations
- **Drain tracking**: Returns information about which messages were overwritten
- **Thread-safe**: Built on flume's proven concurrency primitives
- **Zero-copy**: Minimal overhead over standard flume channels

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
flume-overwrite = "0.1.0"
```

## Usage Examples

### Basic Overwrite Behavior

```rust
use flume_overwrite::bounded;

let (sender, receiver) = bounded(2);

// Fill the channel
assert_eq!(sender.send_overwrite("first").unwrap(), None);
assert_eq!(sender.send_overwrite("second").unwrap(), None);

// This overwrites "first"
let overwritten = sender.send_overwrite("third").unwrap();
assert_eq!(overwritten, Some(vec!["first"]));

// Only "second" and "third" remain
assert_eq!(receiver.recv().unwrap(), "second");
assert_eq!(receiver.recv().unwrap(), "third");
```

### Async Operations

```rust
use flume_overwrite::bounded;
use futures::executor::block_on;

let (sender, receiver) = bounded(1);

block_on(async {
    // Send without overwriting
    assert_eq!(sender.send_overwrite_async("hello").await.unwrap(), None);
    
    // This will overwrite "hello"
    let overwritten = sender.send_overwrite_async("world").await.unwrap();
    assert_eq!(overwritten, Some(vec!["hello"]));
    
    assert_eq!(receiver.recv_async().await.unwrap(), "world");
});
```

### Producer-Consumer with Overwrite

Perfect for scenarios where you want to ensure the consumer always gets the latest data without blocking the producer:

```rust
use flume_overwrite::bounded;
use std::thread;
use std::time::Duration;

let (sender, receiver) = bounded(3);

// Producer thread - never blocks
let producer = thread::spawn(move || {
    for i in 0..10 {
        if let Ok(overwritten) = sender.send_overwrite(i) {
            if let Some(old_values) = overwritten {
                println!("Overwritten values: {:?}", old_values);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
});

// Consumer thread - processes at its own pace
let consumer = thread::spawn(move || {
    while let Ok(value) = receiver.recv() {
        println!("Processing: {}", value);
        thread::sleep(Duration::from_millis(50)); // Simulate slow processing
    }
});

producer.join().unwrap();
// Consumer will process the latest available messages
```

### Integration with Standard Flume Operations

The `OverwriteSender` implements `Deref<Target = Sender<T>>`, so you can use all standard flume sender methods:

```rust
use flume_overwrite::bounded;

let (sender, receiver) = bounded(5);

// Use standard flume methods
sender.send(42).unwrap();
println!("Channel length: {}", sender.len());
println!("Channel capacity: {:?}", sender.capacity());

// Or use overwrite methods
sender.send_overwrite(43).unwrap();
```

## Use Cases

This library is particularly useful for:

- **Real-time data streams**: Sensor data, market feeds, telemetry where latest data is most important
- **Event logging**: Keeping recent events while discarding old ones automatically  
- **Producer-consumer scenarios**: Where producers shouldn't be blocked by slow consumers
- **Buffering latest state**: GUI updates, game state, configuration changes
- **Rate limiting**: Dropping excess messages while preserving recent ones

## Requirements

- Rust 1.75.0 or later (edition 2024)
- Compatible with `no_std` environments (through flume)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.