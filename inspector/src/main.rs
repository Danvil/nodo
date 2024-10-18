use clap::Parser;
use core::time::Duration;
use eyre::Result;
use nodo::{
    codelet::{Transition, TransitionStatistics},
    prelude::DefaultStatus,
};
use nodo_runtime::{InspectorClient, InspectorCodeletReport, InspectorReport, RenderedStatus};
use ratatui::{
    crossterm::event::{self, KeyCode},
    layout::{Constraint, Layout},
    prelude::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};
use regex::Regex;
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value = "tcp://localhost:54399")]
    address: String,

    #[arg(long)]
    disable_tui: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let mut terminal = (!cli.disable_tui).then(|| ratatui::init());

    let inspector = InspectorClient::dial(&cli.address)?;

    let mut rvc = ReportViewController::new();

    // Main loop to handle input events.
    let mut latest_report = None;
    loop {
        if let Some(next) = inspector.try_recv_report()? {
            latest_report = Some(next);
        }

        if let Some(terminal) = terminal.as_mut() {
            terminal.draw(|f| rvc.draw_ui(f, latest_report.as_ref()))?;

            // Exit on "q" key press.
            if event::poll(Duration::from_millis(500))? {
                match event::read()? {
                    event::Event::Key(key) => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down => rvc.select_next(),
                        KeyCode::Up => rvc.select_previous(),
                        KeyCode::Enter => rvc.toggle_expand(),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    ratatui::restore();

    Ok(())
}

struct ReportViewController {
    table_state: TableState,
    expanded_seq: HashMap<String, bool>,
    maybe_selected_seq: Option<String>,
}

impl ReportViewController {
    pub fn new() -> Self {
        Self {
            table_state: TableState::new(),
            expanded_seq: HashMap::new(),
            maybe_selected_seq: None,
        }
    }

    pub fn select_next(&mut self) {
        self.table_state.select_next();
    }

    pub fn select_previous(&mut self) {
        self.table_state.select_previous();
    }

    pub fn toggle_expand(&mut self) {
        if let Some(selected_seq) = self.maybe_selected_seq.as_ref() {
            self.expanded_seq
                .entry(selected_seq.into())
                .and_modify(|v| *v = !*v)
                .or_insert(false);
        }
    }

    // Updated draw_ui to handle the new InspectorReport structure and create a single table.
    pub fn draw_ui(&mut self, frame: &mut Frame, report: Option<&InspectorReport>) {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(frame.area());

        let mut entries = report.map_or_else(|| Vec::new(), |report| report.clone().into_vec());

        // duration of all nodelets
        let overall_step_duration_total: f32 = entries
            .iter()
            .map(|u| {
                u.statistics.transitions[Transition::Step]
                    .duration
                    .total()
                    .as_secs_f32()
            })
            .sum();

        // duration of each nodelet group
        let sequence_duration_sum = compute_sequence_duration_sum(&entries);

        // Sort first by sequence total duration, second by total duration and thirdby name
        entries.sort_by(|a, b| {
            let seq_a = sequence_duration_sum[&a.sequence];
            let seq_b = sequence_duration_sum[&b.sequence];

            let duration_a = a.statistics.transitions[Transition::Step]
                .duration
                .total()
                .as_millis();
            let duration_b = b.statistics.transitions[Transition::Step]
                .duration
                .total()
                .as_millis();

            seq_a
                .partial_cmp(&seq_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| duration_a.cmp(&duration_b))
                .then_with(|| a.name.cmp(&b.name))
        });

        // Create rows for the combined table.
        let mut combined_rows: Vec<_> = Vec::new();
        let mut prev_sequence = None;
        let mut sel_helper = Vec::new();
        for u in entries.into_iter().rev() {
            let seq_duration = sequence_duration_sum[&u.sequence];
            let seq = if u.sequence == "" {
                "(ungrouped)".into()
            } else {
                u.sequence
            };

            let is_expanded = *self.expanded_seq.entry(seq.clone()).or_insert(true);

            if Some(&seq) != prev_sequence.as_ref() {
                prev_sequence = Some(seq.clone());

                let head = Row::new(vec![
                    Cell::from(Span::styled(
                        format!("{}{}", if is_expanded { "+ " } else { "- " }, seq),
                        Color::White,
                    )),
                    Cell::from("--------"),
                    Cell::from("----------"),
                    Cell::from(align_right(format_total_duration(
                        seq_duration,
                        overall_step_duration_total,
                    ))),
                    Cell::from("----------"),
                    Cell::from("----------"),
                    Cell::from("----------"),
                ]);

                combined_rows.push(head);
                sel_helper.push((true, seq.clone()));
            }

            if is_expanded {
                let transition = &u.statistics.transitions[Transition::Step];

                let row = Row::new(vec![
                    Cell::from(Span::styled(format!("├── {}", u.name), Color::White)),
                    Cell::from(format_status(&u.status)),
                    Cell::from(align_right(format_skip_percent(transition))),
                    Cell::from(align_right(format_total_duration(
                        transition.duration.total().as_secs_f32(),
                        overall_step_duration_total,
                    ))),
                    Cell::from(align_right(format_step_count(transition))),
                    Cell::from(align_right(format_period(transition))),
                    Cell::from(Text::from(format_typename(&u.typename))),
                ]);

                combined_rows.push(row);
                sel_helper.push((false, seq.clone()));
            }
        }

        self.maybe_selected_seq = None;
        if let Some(idx) = self.table_state.selected() {
            if let Some((is_head, name)) = sel_helper.get(idx) {
                if *is_head {
                    self.maybe_selected_seq = Some(name.clone());
                }
            }
        }

        // Create the combined table.
        let combined_table = Table::new(
            combined_rows,
            &[
                Constraint::Fill(2),    // Inspector name
                Constraint::Fill(1),    // Status label
                Constraint::Length(8),  // Skipped flag
                Constraint::Length(10), // Total duration
                Constraint::Length(10), // Count
                Constraint::Length(10), // Period
                Constraint::Fill(4),    // Type name
            ],
        )
        .header(
            Row::new(vec![
                "Codelet", "Status", "Skip%", "Time", "Count", "Period", "Type",
            ])
            .style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                " NODO INSPECTOR ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .style(Color::Yellow);

        // Render the combined table.
        frame.render_stateful_widget(combined_table, chunks[0], &mut self.table_state);
    }
}

fn compute_sequence_duration_sum(reports: &[InspectorCodeletReport]) -> HashMap<String, f32> {
    let mut sequence_duration_map = HashMap::new();

    for report in reports {
        // Access the Transition::Step for each report's statistics
        let step_transition = &report.statistics.transitions[Transition::Step];

        // Get the total duration in seconds
        let duration_secs = step_transition.duration.total().as_secs_f32();

        // Add the duration to the corresponding sequence
        sequence_duration_map
            .entry(report.sequence.clone())
            .and_modify(|e| *e += duration_secs)
            .or_insert(duration_secs);
    }

    sequence_duration_map
}

fn align_right(span: Span<'_>) -> Text<'_> {
    Text::from(span).alignment(Alignment::Right)
}

fn format_status(maybe_status: &Option<RenderedStatus>) -> Span<'static> {
    if let Some(status) = maybe_status {
        let status_style = if status.status == DefaultStatus::Skipped {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        Span::styled(status.label.clone(), status_style)
    } else {
        Span::styled("None", Color::DarkGray)
    }
}

fn format_skip_percent(u: &TransitionStatistics) -> Span<'static> {
    if u.skipped_count == 0 {
        Span::styled(format!("{:>6}", "None"), Color::DarkGray)
    } else {
        let p = u.skip_percent();
        let color = if p < 0.8 {
            Color::White
        } else if p < 0.9 {
            Color::Yellow
        } else {
            Color::LightRed
        };
        Span::styled(format!("{:>5.1}%", p * 100.), color)
    }
}

fn format_total_duration(x: f32, overall_total: f32) -> Span<'static> {
    let p = x / overall_total;
    let color = if p > 0.10 {
        Color::LightRed
    } else if p > 0.01 {
        Color::Yellow
    } else {
        Color::White
    };
    Span::styled(format!("{:>7.1}s", x), color)
}

fn format_step_count(u: &TransitionStatistics) -> Span<'static> {
    let x = u.duration.count();
    Span::styled(format!("{:>8}", x), Color::White)
}

fn format_period(u: &TransitionStatistics) -> Span<'static> {
    if let Some(x) = u.period.average_ms() {
        Span::styled(format!("{:>6.1} ms", x), Color::White)
    } else {
        Span::styled(format!("{:>8}", "Never"), Color::DarkGray)
    }
}

/// Function to format a string as a `Span` with color formatting.
fn format_typename<'a>(input: &str) -> Line<'a> {
    // Define a regex to match the format [namespace::]typename[<generics>]
    let regex = Regex::new(r"(?P<namespace>(?:[a-zA-Z_][a-zA-Z0-9_]*::)+)?(?P<typename>[a-zA-Z_][a-zA-Z0-9_]*)?(?P<generics><.+>)?")
        .unwrap();

    // This will hold the formatted spans.
    let mut spans = Vec::new();

    // If the regex matches the input, split it into namespace, typename, and generics.
    if let Some(captures) = regex.captures(input) {
        // Extract namespace if present
        if let Some(namespace) = captures.name("namespace") {
            spans.push(Span::styled(
                namespace.as_str().to_string(),
                Style::default().fg(Color::Blue),
            ));
        }

        // Extract typename
        if let Some(typename) = captures.name("typename") {
            spans.push(Span::styled(
                typename.as_str().to_string(),
                Style::default().fg(Color::White),
            ));
        }

        // Extract generics if present
        if let Some(generics) = captures.name("generics") {
            spans.push(Span::styled(
                generics.as_str().to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    spans.into()
}
