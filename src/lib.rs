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
