// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::codelet::Manifold;
use crate::codelet::Transition;
use crate::codelet::TransitionMap;
use crate::codelet::VertexId;
use core::time::Duration;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Default, Clone)]
pub struct Statistics(pub HashMap<VertexId, VertexStatistics>);

#[derive(Debug, Default, Clone)]
pub struct VertexStatistics(pub TransitionMap<TransitionStatistics>);

#[derive(Default, Debug, Clone)]
pub struct TransitionStatistics {
    pub duration: CountTotal,
    pub period: CountTotal,
    pub skipped_count: u64,
    last_exec_begin: Option<Instant>,
}

#[derive(Default, Debug, Clone)]
pub struct CountTotal {
    count: u64,
    total: Duration,
    limits: (Duration, Duration),
}

impl VertexStatistics {
    pub fn new() -> Self {
        Self(TransitionMap::default())
    }
}

impl TransitionStatistics {
    pub fn new() -> Self {
        Self {
            duration: CountTotal::default(),
            period: CountTotal::default(),
            skipped_count: 0,
            last_exec_begin: None,
        }
    }

    pub fn begin(&mut self) {
        let now = Instant::now();

        if let Some(last_exec) = self.last_exec_begin {
            self.period.push(now - last_exec);
        }

        self.last_exec_begin = Some(now);
    }

    pub fn end(&mut self, skipped: bool) {
        if skipped {
            self.skipped_count += 1;
        } else {
            self.duration.push(
                Instant::now()
                    - self
                        .last_exec_begin
                        .expect("end() must be called after begin()"),
            );
        }
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

pub fn statistics_pretty_print(manifold: &Manifold, stats: Statistics) {
    let mut vec = stats.0.iter().collect::<Vec<_>>();
    vec.sort_by_key(|(_, stats)| stats.0[Transition::Step].duration.total.as_nanos());

    println!("");
    println!("+--------------------------+----------------------------------+--------+--------+----------------------+-------+----------------------+--------+---------+");
    println!("| NAME                     | TYPE                             | STEP              Duration                       Period               | START            |");
    println!("|                          |                                  | Skipped| Count  | (min-avg-max) [ms]   | Total | (min-avg-max) [ms]   | Count  |  D [ms] |");
    println!("+--------------------------+----------------------------------+--------+--------+----------------------+-------+----------------------+--------+---------+");
    for (vid, stats) in vec.into_iter().rev() {
        let vertex = &manifold[*vid];

        println!(
            "| {:024} | {:032} | {:6} | {:6} | {} {} {} |{} | {} {} {} | {:2} /{:2} | {} |",
            cut_middle(&vertex.name, 24),
            cut_middle(&vertex.typename, 32),
            stats.0[Transition::Step].skipped_count,
            stats.0[Transition::Step].duration.count(),
            stats.0[Transition::Step]
                .duration
                .min_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.0[Transition::Step]
                .duration
                .average_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.0[Transition::Step]
                .duration
                .max_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            format!(
                "{:>6.2}",
                stats.0[Transition::Step].duration.total.as_secs_f32()
            ),
            stats.0[Transition::Step]
                .period
                .min_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.0[Transition::Step]
                .period
                .average_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.0[Transition::Step]
                .period
                .max_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.0[Transition::Start].skipped_count,
            stats.0[Transition::Start].duration.count(),
            stats.0[Transition::Start]
                .duration
                .average_ms()
                .map(|dt| format!("{:>7.2}", dt))
                .unwrap_or("-------".to_string()),
        );
    }
    println!("+--------------------------+----------------------------------+--------+--------+----------------------+-------+----------------------+--------+---------+");
}

fn cut_middle(text: &String, len: usize) -> String {
    if text.len() <= len || len <= 6 {
        text.to_string()
    } else {
        text[0..2].to_string() + ".." + &text[(text.len() - (len - 4))..]
    }
}
