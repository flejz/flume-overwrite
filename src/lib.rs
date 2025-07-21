use flume::{Receiver, SendError, Sender};
use std::ops::Deref;

pub fn bounded_overwrite<T>(cap: usize) -> (OverwriteSender<T>, Receiver<T>) {
    let (tx, rx) = flume::bounded(cap);
    let overwrite_sender = OverwriteSender {
        sender: tx,
        receiver: rx.clone(),
    };
    (overwrite_sender, rx)
}

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
