// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::{InspectorCodeletReport, InspectorReport};
use nodo::codelet::Transition;

pub fn statistics_pretty_print(report: InspectorReport) {
    let mut vec = report.into_vec();
    vec.sort_by_key(|(_, u)| {
        u.statistics.transitions[Transition::Step]
            .duration
            .total()
            .as_nanos()
    });

    println!("");
    println!("+--------------------------+----------------------------------+--------+--------+----------------------+-------+----------------------+--------+---------+");
    println!("| NAME                     | TYPE                             | STEP              Duration                       Period               | START            |");
    println!("|                          |                                  | Skipped| Count  | (min-avg-max) [ms]   | Total | (min-avg-max) [ms]   | Count  |  D [ms] |");
    println!("+--------------------------+----------------------------------+--------+--------+----------------------+-------+----------------------+--------+---------+");
    for (
        _,
        InspectorCodeletReport {
            name: tag,
            typename,
            statistics: stats,
            ..
        },
    ) in vec.into_iter().rev()
    {
        println!(
            "| {:024} | {:032} | {:6} | {:6} | {} {} {} |{} | {} {} {} | {:2} /{:2} | {} |",
            cut_middle(&tag, 24),
            cut_middle(&typename, 32),
            stats.transitions[Transition::Step].skipped_count,
            stats.transitions[Transition::Step].duration.count(),
            stats.transitions[Transition::Step]
                .duration
                .min_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.transitions[Transition::Step]
                .duration
                .average_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.transitions[Transition::Step]
                .duration
                .max_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            format!(
                "{:>6.2}",
                stats.transitions[Transition::Step]
                    .duration
                    .total()
                    .as_secs_f32()
            ),
            stats.transitions[Transition::Step]
                .period
                .min_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.transitions[Transition::Step]
                .period
                .average_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.transitions[Transition::Step]
                .period
                .max_ms()
                .map(|dt| format!("{:>6.2}", dt))
                .unwrap_or("------".to_string()),
            stats.transitions[Transition::Start].skipped_count,
            stats.transitions[Transition::Start].duration.count(),
            stats.transitions[Transition::Start]
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
