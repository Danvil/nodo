use clap::Parser;
use core::time::Duration;
use eyre::Result;
use nodo::{
    codelet::{Transition, TransitionStatistics},
    prelude::DefaultStatus,
};
use nodo_runtime::{InspectorClient, InspectorReport, RenderedStatus};
use ratatui::{
    crossterm::event::{self, KeyCode},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use regex::Regex;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value = "tcp://localhost:54399")]
    address: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Setup terminal.
    let mut terminal = ratatui::init();

    let inspector = InspectorClient::dial(&cli.address)?;

    // Main loop to handle input events.
    let mut latest_report = None;
    loop {
        if let Some(next) = inspector.try_recv_report()? {
            latest_report = Some(next);
        }

        terminal.draw(|f| draw_ui(f, latest_report.as_ref()))?;

        // Exit on "q" key press.
        if event::poll(Duration::from_millis(500))? {
            if let event::Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Restore terminal.
    ratatui::restore();

    Ok(())
}

// Updated draw_ui to handle the new InspectorReport structure and create a single table.
fn draw_ui(f: &mut Frame, report: Option<&InspectorReport>) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.area());

    // Sort first by total duration and second by name
    let mut entries_sorted = report.map_or_else(|| Vec::new(), |report| report.clone().into_vec());
    entries_sorted.sort_by(|a, b| {
        let duration_a = a.statistics.transitions[Transition::Step]
            .duration
            .total()
            .as_millis();
        let duration_b = b.statistics.transitions[Transition::Step]
            .duration
            .total()
            .as_millis();

        // First compare by total duration, if equal, compare by name.
        duration_a
            .cmp(&duration_b)
            .then_with(|| a.name.cmp(&b.name))
    });
    let overall_step_duration_total: f32 = entries_sorted
        .iter()
        .map(|u| {
            u.statistics.transitions[Transition::Step]
                .duration
                .total()
                .as_secs_f32()
        })
        .sum();

    // Create rows for the combined table.
    let combined_rows: Vec<Row> = entries_sorted
        .into_iter()
        .rev()
        .map(|codelet_report| {
            // For each `codelet_report`, create one row for each transition in statistics.
            let transition = &codelet_report.statistics.transitions[Transition::Step];

            Row::new(vec![
                Cell::from(Span::styled(codelet_report.name, Color::White)),
                Cell::from(format_status(&codelet_report.status)),
                Cell::from(format_skip_percent(transition)),
                Cell::from(format_total_duration(
                    transition,
                    overall_step_duration_total,
                )),
                Cell::from(format_step_count(transition)),
                Cell::from(format_period(transition)),
                Cell::from(Text::from(format_typename(&codelet_report.typename))),
            ])
        })
        .collect();

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
    .style(Color::Yellow);

    // Render the combined table.
    f.render_widget(combined_table, chunks[0]);
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

fn format_total_duration(u: &TransitionStatistics, overall_total: f32) -> Span<'static> {
    let x = u.duration.total().as_secs_f32();
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
