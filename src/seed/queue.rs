use anyhow::Result;
use flume::{Receiver, Sender};

/// A queue that can be used to send and receive values asynchronously
pub struct SeedQueue<T> {
    queue: Receiver<T>
}

impl<T> SeedQueue<T> {
    /// Creates a new `SeedQueue` with optional bounded capacity
    ///
    /// # Arguments
    /// * `bound` - Optional size limit for the queue. If `None`, creates an unbounded queue
    ///
    /// # Returns
    /// Returns a tuple containing the queue and a sender that can be used to send values to it.
    /// The sender can be freely cloned and shared between threads/tasks.
    pub fn new(bound: Option<usize>) -> (Self, Sender<T>) {
        let (tx, rx) = match bound {
            Some(cap) => flume::bounded::<T>(cap),
            None => flume::unbounded()
        };

        let obj = Self {
            queue: rx
        };

        (obj, tx)
    }

    /// Asynchronously receives a value from the queue
    ///
    /// # Returns
    /// Returns a Result containing the received value, or an error if the receive operation fails
    ///
    /// # Errors
    /// Will return an error if the sender is disconnected or if the receive operation fails
    pub async fn receive(&self) -> Result<T> {
        self.queue.recv_async().await.map_err(|e| anyhow::anyhow!(e))   
    }
}