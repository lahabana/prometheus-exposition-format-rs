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

#[derive(Debug, PartialEq)]
pub struct Sample {
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: Option<i64>,
}

impl Sample {
    pub fn new(value: f64, timestamp: Option<i64>, labels: Vec<&str>) -> Self {
        let labels = labels
            .iter()
            .enumerate()
            .step_by(2)
            .map(|(i, _)| (labels[i].to_string(), labels[i + 1].to_string()))
            .collect();
        Sample {
            labels,
            value,
            timestamp,
        }
    }
}

#[derive(Debug)]
pub struct Metric {
    pub name: String,
    pub data_type: MetricType,
    pub samples: Vec<Sample>,
}

impl Metric {
    pub fn new(name: &str, t: MetricType) -> Self {
        Metric {
            name: name.to_string(),
            data_type: t,
            samples: Vec::new(),
        }
    }

    pub fn push_sample(&mut self, s: Sample) {
        self.samples.push(s);
    }
}
