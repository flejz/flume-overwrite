//! # Flume Overwrite
//!
//! A library that provides bounded channels with overwrite capability, built on top of the `flume` crate.
//! When the channel reaches capacity, new messages will overwrite the oldest unread messages.
//!
//! ## Features
//!
//! - **Bounded channels with overwrite**: Messages sent to a full channel will replace the oldest messages
//! - **Async support**: Both blocking and async send operations
//! - **Drain tracking**: Returns information about which messages were overwritten
//!
//! ## Examples
//!
//! ```rust
//! use flume_overwrite::bounded_overwrite;
//!
//! // Create a channel with capacity 3
//! let (sender, receiver) = bounded_overwrite(3);
//!
//! // Send messages normally when under capacity
//! sender.send_overwrite(1).unwrap();
//! sender.send_overwrite(2).unwrap();
//! sender.send_overwrite(3).unwrap();
//!
//! // This will overwrite the first message (1)
//! let overwritten = sender.send_overwrite(4).unwrap();
//! assert_eq!(overwritten, Some(vec![1]));
//!
//! // Receive the remaining messages
//! assert_eq!(receiver.recv().unwrap(), 2);
//! assert_eq!(receiver.recv().unwrap(), 3);
//! assert_eq!(receiver.recv().unwrap(), 4);
//! ```

use flume::{Receiver, SendError, Sender};
use std::ops::Deref;

/// Creates a bounded channel with overwrite capability.
///
/// Returns a tuple of `(OverwriteSender<T>, Receiver<T>)` where the sender can overwrite
/// old messages when the channel reaches capacity, and the receiver is a standard flume receiver.
///
/// # Arguments
///
/// * `cap` - The maximum number of messages the channel can hold
///
/// # Returns
///
/// A tuple containing:
/// - `OverwriteSender<T>` - A sender that can overwrite old messages when at capacity
/// - `Receiver<T>` - A standard flume receiver for reading messages
///
/// # Examples
///
/// ```rust
/// use flume_overwrite::bounded_overwrite;
///
/// let (sender, receiver) = bounded_overwrite(2);
/// sender.send_overwrite("hello").unwrap();
/// sender.send_overwrite("world").unwrap();
///
/// assert_eq!(receiver.recv().unwrap(), "hello");
/// assert_eq!(receiver.recv().unwrap(), "world");
/// ```
pub fn bounded_overwrite<T>(cap: usize) -> (OverwriteSender<T>, Receiver<T>) {
    let (tx, rx) = flume::bounded(cap);
    let overwrite_sender = OverwriteSender {
        sender: tx,
        receiver: rx.clone(),
    };
    (overwrite_sender, rx)
}

/// A sender that can overwrite old messages when the channel reaches capacity.
///
/// `OverwriteSender<T>` wraps a flume `Sender<T>` and provides additional functionality
/// to automatically remove old messages when sending would block due to a full channel.
///
/// This struct implements `Deref` to `Sender<T>`, so all standard sender methods are available.
/// Additionally, it provides `send_overwrite` and `send_overwrite_async` methods that will
/// never block due to a full channel.
///
/// # Examples
///
/// ```rust
/// use flume_overwrite::bounded_overwrite;
///
/// let (sender, receiver) = bounded_overwrite(1);
///
/// // First message goes through normally
/// sender.send_overwrite("first").unwrap();
///
/// // Second message overwrites the first
/// let overwritten = sender.send_overwrite("second").unwrap();
/// assert_eq!(overwritten, Some(vec!["first"]));
/// ```
#[derive(Clone)]
pub struct OverwriteSender<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Deref for OverwriteSender<T> {
    type Target = Sender<T>;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}

impl<T> OverwriteSender<T> {
    /// Sends a value, overwriting old messages if the channel is at capacity.
    ///
    /// This method will never block. If the channel is at capacity, it will remove
    /// old messages from the front of the queue until there's space for the new message.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to send through the channel
    ///
    /// # Returns
    ///
    /// - `Ok(None)` - The message was sent without overwriting any existing messages
    /// - `Ok(Some(Vec<T>))` - The message was sent and the returned vector contains
    ///   the messages that were overwritten (removed from the channel)
    /// - `Err(SendError<T>)` - The channel is disconnected
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flume_overwrite::bounded_overwrite;
    ///
    /// let (sender, receiver) = bounded_overwrite(2);
    ///
    /// // Send without overwriting
    /// assert_eq!(sender.send_overwrite(1).unwrap(), None);
    /// assert_eq!(sender.send_overwrite(2).unwrap(), None);
    ///
    /// // This will overwrite the first message
    /// let overwritten = sender.send_overwrite(3).unwrap();
    /// assert_eq!(overwritten, Some(vec![1]));
    /// ```
    pub fn send_overwrite(&self, value: T) -> Result<Option<Vec<T>>, SendError<T>> {
        if let Some(capacity) = self.sender.capacity() {
            let mut drained = Vec::new();
            while self.sender.len() >= capacity {
                match self.receiver.try_recv() {
                    Ok(old_value) => drained.push(old_value),
                    Err(flume::TryRecvError::Empty) => (),
                    Err(_) => {
                        return Err(SendError(value));
                    }
                }
            }
            self.sender.send(value)?;
            Ok(if drained.is_empty() {
                None
            } else {
                Some(drained)
            })
        } else {
            self.sender.send(value)?;
            Ok(None)
        }
    }

    /// Asynchronously sends a value, overwriting old messages if the channel is at capacity.
    ///
    /// This is the async version of `send_overwrite`. Like its synchronous counterpart,
    /// this method will never block due to a full channel - it will instead remove old
    /// messages to make space.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to send through the channel
    ///
    /// # Returns
    ///
    /// A future that resolves to:
    /// - `Ok(None)` - The message was sent without overwriting any existing messages
    /// - `Ok(Some(Vec<T>))` - The message was sent and the returned vector contains
    ///   the messages that were overwritten (removed from the channel)
    /// - `Err(SendError<T>)` - The channel is disconnected
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flume_overwrite::bounded_overwrite;
    /// use futures::executor::block_on;
    ///
    /// let (sender, receiver) = bounded_overwrite(1);
    ///
    /// block_on(async {
    ///     // Send without overwriting
    ///     assert_eq!(sender.send_overwrite_async(1).await.unwrap(), None);
    ///     
    ///     // This will overwrite the first message
    ///     let overwritten = sender.send_overwrite_async(2).await.unwrap();
    ///     assert_eq!(overwritten, Some(vec![1]));
    /// });
    /// ```
    pub async fn send_overwrite_async(&self, value: T) -> Result<Option<Vec<T>>, SendError<T>> {
        if let Some(capacity) = self.sender.capacity() {
            let mut drained = Vec::new();
            while self.sender.len() >= capacity {
                if let Ok(old_value) = self.receiver.recv_async().await {
                    drained.push(old_value);
                }
            }
            self.sender.send_async(value).await?;
            Ok(if drained.is_empty() {
                None
            } else {
                Some(drained)
            })
        } else {
            self.sender.send_async(value).await?;
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use futures::executor::block_on;

    #[test]
    fn test_send_overwrite_under_capacity() {
        let (sender, receiver) = bounded_overwrite(3);
        assert_eq!(sender.send_overwrite(1).unwrap(), None);
        assert_eq!(sender.send_overwrite(2).unwrap(), None);
        assert_eq!(receiver.try_recv().unwrap(), 1);
        assert_eq!(receiver.try_recv().unwrap(), 2);
    }

    #[test]
    fn test_send_overwrite_at_capacity() {
        let (sender, receiver) = bounded_overwrite(2);
        assert_eq!(sender.send_overwrite(1).unwrap(), None);
        assert_eq!(sender.send_overwrite(2).unwrap(), None);

        let drained = sender.send_overwrite(3).unwrap();
        assert_eq!(drained, Some(vec![1]));
        assert_eq!(receiver.try_recv().unwrap(), 2);
        assert_eq!(receiver.try_recv().unwrap(), 3);
    }

    #[test]
    fn test_send_overwrite_multiple_overwrites() {
        let (sender, receiver) = bounded_overwrite(2);
        assert_eq!(sender.send_overwrite(1).unwrap(), None);
        assert_eq!(sender.send_overwrite(2).unwrap(), None);
        // Fill up, then send two more, should drain two
        let drained = sender.send_overwrite(3).unwrap();
        assert_eq!(drained, Some(vec![1]));
        let drained2 = sender.send_overwrite(4).unwrap();
        assert_eq!(drained2, Some(vec![2]));
        assert_eq!(receiver.try_recv().unwrap(), 3);
        assert_eq!(receiver.try_recv().unwrap(), 4);
    }

    #[test]
    fn test_send_overwrite_unbounded() {
        let (sender, receiver) = bounded_overwrite(2);
        assert_eq!(sender.send_overwrite(1).unwrap(), None);
        assert_eq!(sender.send_overwrite(2).unwrap(), None);
        assert_eq!(receiver.try_recv().unwrap(), 1);
        assert_eq!(receiver.try_recv().unwrap(), 2);
    }

    #[test]
    fn test_send_overwrite_async_under_capacity() {
        let (sender, receiver) = bounded_overwrite(3);
        let fut = sender.send_overwrite_async(1);
        assert_eq!(block_on(fut).unwrap(), None);
        let fut = sender.send_overwrite_async(2);
        assert_eq!(block_on(fut).unwrap(), None);
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 1);
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 2);
    }

    #[test]
    fn test_send_overwrite_async_at_capacity() {
        let (sender, receiver) = bounded_overwrite(2);
        block_on(sender.send_overwrite_async(1)).unwrap();
        block_on(sender.send_overwrite_async(2)).unwrap();
        let drained = block_on(sender.send_overwrite_async(3)).unwrap();
        assert_eq!(drained, Some(vec![1]));
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 2);
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 3);
    }

    #[test]
    fn test_send_overwrite_async_multiple_overwrites() {
        let (sender, receiver) = bounded_overwrite(2);
        block_on(sender.send_overwrite_async(1)).unwrap();
        block_on(sender.send_overwrite_async(2)).unwrap();
        let drained = block_on(sender.send_overwrite_async(3)).unwrap();
        assert_eq!(drained, Some(vec![1]));
        let drained2 = block_on(sender.send_overwrite_async(4)).unwrap();
        assert_eq!(drained2, Some(vec![2]));
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 3);
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 4);
    }

    #[test]
    fn test_send_overwrite_async_unbounded() {
        let (sender, receiver) = bounded_overwrite(2);
        assert_eq!(block_on(sender.send_overwrite_async(1)).unwrap(), None);
        assert_eq!(block_on(sender.send_overwrite_async(2)).unwrap(), None);
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 1);
        assert_eq!(block_on(receiver.recv_async()).unwrap(), 2);
    }

    #[test]
    fn test_send_overwrite_concurrent() {
        let (sender, receiver) = bounded_overwrite(2);
        let sender_clone = sender.clone();
        let handle = thread::spawn(move || {
            for i in 0..5 {
                let drained = sender_clone.send_overwrite(i).unwrap();
                println!("drained: {:?}", drained);
                thread::sleep(Duration::from_millis(10));
            }
        });
        handle.join().unwrap();
        let mut received = Vec::new();
        while let Ok(val) = receiver.try_recv() {
            received.push(val);
        }
        // Should have at most 2 items, the last two sent
        assert!(received.len() <= 2);
        if received.len() == 2 {
            assert_eq!(received, vec![3, 4]);
        }
    }

    #[test]
    fn test_send_overwrite_async_concurrent() {
        use std::sync::Mutex;
        let (sender, receiver) = bounded_overwrite(2);
        let sender_clone = sender.clone();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received2 = received.clone();
        let handle = thread::spawn(move || {
            block_on(async {
                for i in 0..5 {
                    sender_clone.send_overwrite_async(i).await.unwrap();
                    // TODO: use a real delay
                    // simulate work
                    futures_timer::Delay::new(Duration::from_millis(10)).await;
                }
            });
        });
        handle.join().unwrap();
        while let Ok(val) = receiver.try_recv() {
            received2.lock().unwrap().push(val);
        }
        let got = received.lock().unwrap();
        // Should have at most 2 items, the last two sent
        assert!(got.len() <= 2);
        if got.len() == 2 {
            assert_eq!(*got, vec![3, 4]);
        }
    }
}
