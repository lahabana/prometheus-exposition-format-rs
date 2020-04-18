use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum MetricType {
    Untyped,
    Counter,
    Gauge,
    Histogram,
    Summary,
}

type NomErr<A> = nom::Err<(A, nom::error::ErrorKind)>;

#[derive(Debug)]
pub struct Err(String);

impl From<NomErr<&str>> for Err {
    fn from(t: NomErr<&str>) -> Self {
        Err(format!("{:?}", t))
    }
}

#[derive(Debug)]
pub struct Sample {
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: Option<i64>,
}

#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub data_type: MetricType,
    pub samples: Vec<Sample>,
    pub help: Option<String>,
}


impl Metric {
    pub fn new(name: &str, t: MetricType) -> Self {
        Metric {
            name: name.to_string(),
            data_type: t,
            samples: Vec::new(),
            help: None,
        }
    }

    pub fn push_sample(&mut self, s: Sample) {
        self.samples.push(s);
    }
}