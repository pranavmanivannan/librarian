use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

lazy_static! {
    pub static ref COUNTER: Arc<Counter> = Arc::new(Counter::new());
}

/// A simple counter struct containing an AtomicUsize. It is mainly used to keep track of the number of messages
/// ingested by all the buffers.
#[derive(Debug)]
pub struct Counter {
    value: AtomicUsize,
}

impl Counter {
    /// Simple constructor for the Counter struct.
    ///
    /// # Returns
    /// A Counter struct with a value of 0.
    pub fn new() -> Self {
        Counter {
            value: AtomicUsize::new(0),
        }
    }

    /// Increments the counter by 1.
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }

    /// Gets the value of the counter.
    ///
    /// # Returns
    /// The counter's value as a usize.
    pub fn get_value(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }

    /// Resets the value of the counter to 0.
    pub fn reset(&self) {
        self.value.store(0, Ordering::SeqCst);
    }
}
