// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::Transition;
use crate::codelet::TransitionMap;
use core::time::Duration;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Statistics {
    pub transitions: TransitionMap<TransitionStatistics>,
}

#[derive(Default, Debug, Clone)]
pub struct TransitionStatistics {
    pub duration: CountTotal,
    pub period: CountTotal,
    last_exec_begin: Option<Instant>,
}

#[derive(Default, Debug, Clone)]
pub struct CountTotal {
    count: u64,
    total: Duration,
    limits: (Duration, Duration),
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            transitions: TransitionMap::default(),
        }
    }
}

impl TransitionStatistics {
    pub fn new() -> Self {
        Self {
            duration: CountTotal::default(),
            last_exec_begin: None,
            period: CountTotal::default(),
        }
    }

    pub fn begin(&mut self) {
        let now = Instant::now();

        if let Some(last_exec) = self.last_exec_begin {
            self.period.push(now - last_exec);
        }

        self.last_exec_begin = Some(now);
    }

    pub fn end(&mut self) {
        self.duration.push(
            Instant::now()
                - self
                    .last_exec_begin
                    .expect("end() must be called after begin()"),
        );
    }
}

impl CountTotal {
    pub fn push(&mut self, dt: Duration) {
        self.count += 1;
        self.total += dt;
        self.limits = if self.count == 1 {
            (dt, dt)
        } else {
            (self.limits.0.min(dt), self.limits.1.max(dt))
        };
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn average_ms(&self) -> Option<f32> {
        if self.count <= 0 {
            None
        } else {
            Some(self.total.as_secs_f32() * 1000.0 / (self.count as f32))
        }
    }

    pub fn min_ms(&self) -> Option<f32> {
        if self.count <= 0 {
            None
        } else {
            Some(self.limits.0.as_secs_f32() * 1000.0)
        }
    }

    pub fn max_ms(&self) -> Option<f32> {
        if self.count <= 0 {
            None
        } else {
            Some(self.limits.1.as_secs_f32() * 1000.0)
        }
    }
}

pub fn statistics_pretty_print(stats: HashMap<(String, String), Statistics>) {
    println!("");
    println!("+--------------------------+----------------------------------+--------+----------------------+----------------------+--------+---------+");
    println!("| NAME                     | TYPE                             | STEP     Duration               Period               | START            |");
    println!("|                          |                                  | Count  | (min-avg-max) [ms]   | (min-avg-max) [ms]   | Count  |  D [ms] |");
    println!("+--------------------------+----------------------------------+--------+----------------------+----------------------+--------+---------+");
    for ((tag, typename), stats) in stats.iter() {
        println!(
            "| {:024} | {:032} | {:6} | {}-{}-{} | {}-{}-{} | {:6} | {} |",
            cut_middle(tag, 24),
            cut_middle(typename, 32),
            stats.transitions[Transition::Step].duration.count(),
            stats.transitions[Transition::Step]
                .duration
                .min_ms()
                .map(|dt| format!("{:->6.2}", dt))
                .unwrap_or("N/A".to_string()),
            stats.transitions[Transition::Step]
                .duration
                .average_ms()
                .map(|dt| format!("{:->6.2}", dt))
                .unwrap_or("N/A".to_string()),
            stats.transitions[Transition::Step]
                .duration
                .max_ms()
                .map(|dt| format!("{:->6.2}", dt))
                .unwrap_or("N/A".to_string()),
            stats.transitions[Transition::Step]
                .period
                .min_ms()
                .map(|dt| format!("{:->6.2}", dt))
                .unwrap_or("N/A".to_string()),
            stats.transitions[Transition::Step]
                .period
                .average_ms()
                .map(|dt| format!("{:->6.2}", dt))
                .unwrap_or("N/A".to_string()),
            stats.transitions[Transition::Step]
                .period
                .max_ms()
                .map(|dt| format!("{:->6.2}", dt))
                .unwrap_or("N/A".to_string()),
            stats.transitions[Transition::Start].duration.count(),
            stats.transitions[Transition::Start]
                .duration
                .average_ms()
                .map(|dt| format!("{:>7.2}", dt))
                .unwrap_or("N/A".to_string()),
        );
    }
    println!("+--------------------------+----------------------------------+--------+----------------------+----------------------+--------+---------+");
}

fn cut_middle(text: &String, len: usize) -> String {
    if text.len() <= len || len <= 6 {
        text.to_string()
    } else {
        text[0..2].to_string() + ".." + &text[(text.len() - (len - 4))..]
    }
}
