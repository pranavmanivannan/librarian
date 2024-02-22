use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref COUNTER: Arc<Counter> = Arc::new(Counter::new());
}

#[derive(Debug)]
pub struct Counter {
    value: AtomicUsize,
}

impl Counter {
    pub fn new() -> Self {
        Counter {
            value: AtomicUsize::new(0),
        }
    }

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_value(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::SeqCst);
    }
}