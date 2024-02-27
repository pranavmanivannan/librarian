use std::sync::atomic::{AtomicU16, AtomicUsize, Ordering};

pub struct MetricManager {
    pub throughput: ThroughputMetric,
    pub parsetime: ParseMetric,
    pub packetsize: PacketMetric,
}

impl MetricManager {
    pub fn new() -> Self {
        MetricManager {
            throughput: ThroughputMetric::new(),
            parsetime: ParseMetric::new(),
            packetsize: PacketMetric::new(),
        }
    }
}

pub(crate) trait Metric {
    fn calculate(&self) -> f64;
    fn update(&self, value: u16);
    fn log(&self);
}

pub struct ThroughputMetric {
    value: AtomicUsize,
}

impl ThroughputMetric {
    pub fn new() -> Self {
        ThroughputMetric {
            value: AtomicUsize::new(0),
        }
    }
}

impl Metric for ThroughputMetric {
    fn calculate(&self) -> f64 {
        (self.value.load(Ordering::SeqCst) as f64) / 30.0
    }
    
    fn update(&self, _value: u16) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }
    
    fn log(&self) {
        log::info!("throughput: {:?}", self.calculate());
        self.value.store(0, Ordering::SeqCst);
    }
}

pub struct ParseMetric {
    value: AtomicU16,
    count: AtomicU16,
}

impl ParseMetric {
    pub fn new() -> Self {
        ParseMetric {
            value: AtomicU16::new(0),
            count: AtomicU16::new(0),
        }
    }
}

impl Metric for ParseMetric {
    fn calculate(&self) -> f64 {
        (self.value.load(Ordering::SeqCst) as f64) / (self.count.load(Ordering::SeqCst) as f64)
    }
    
    fn update(&self, value: u16) {
        self.value.fetch_add(value, Ordering::SeqCst);
        self.count.fetch_add(1, Ordering::SeqCst);
    }
    
    fn log(&self) {
        log::info!("Average Parse Time: {:?}", self.calculate());
        self.value.store(0, Ordering::SeqCst);
        self.count.store(0, Ordering::SeqCst);
    }
}


pub struct PacketMetric {
    value: AtomicU16,
    count: AtomicU16,
}

impl PacketMetric {
    pub fn new() -> Self {
        PacketMetric {
            value: AtomicU16::new(0),
            count: AtomicU16::new(0),
        }
    }
}

impl Metric for PacketMetric {
    fn calculate(&self) -> f64 {
        (self.value.load(Ordering::SeqCst) as f64) / (self.count.load(Ordering::SeqCst) as f64)
    }
    
    fn update(&self, value: u16) {
        self.value.fetch_add(value, Ordering::SeqCst);
        self.count.fetch_add(1, Ordering::SeqCst);
    }
    
    fn log(&self) {
        log::info!("Average Packet Size: {:?}", self.calculate());
        self.value.store(0, Ordering::SeqCst);
        self.count.store(0, Ordering::SeqCst);
    }
}