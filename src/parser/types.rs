use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum MetricType {
    Untyped,
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Nan,
    PosInf,
    NegInf,
    Value(f32),
}

#[derive(Debug)]
pub struct Metric<'a> {
    labels: &'a HashMap<String, String>,
    value: Value,
    help: Option<String>,
}
