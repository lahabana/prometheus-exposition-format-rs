use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, PartialEq)]
pub enum MetricType {
    Untyped,
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug)]
pub struct Sample {
    labels: HashMap<String, String>,
    value: f64,
    timestamp: Option<Instant>,
}

#[derive(Debug)]
pub struct Metric {
    name: String,
    data_type: MetricType,
    samples: Vec<Sample>,
    help: Option<String>,
}
