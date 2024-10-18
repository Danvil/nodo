use clap::Parser;
use core::time::Duration;
use eyre::Result;
use nodo::codelet::Transition;
use nodo_runtime::InspectorClient;
use nodo_runtime::InspectorReport;
use ratatui::crossterm::event::{self, KeyCode};
use ratatui::Frame;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value = "tcp://localhost:54399")]
    address: String,
}

fn main() -> Result<()> {
    env_logger::init();

    log::info!("FARM FURROW ASSIST VIS");

    let cli = Cli::parse();

    // Setup terminal.
    let mut terminal = ratatui::init();

    let inspector = InspectorClient::dial(&cli.address)?;

    // Main loop to handle input events.
    let mut latest_report = None;
    loop {
        latest_report = inspector.try_recv_report()?;

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

    // Create rows for the combined table.
    let combined_rows: Vec<Row> = report.map_or_else(
        || Vec::new(),
        |report| {
            report
                .clone()
                .into_vec()
                .into_iter()
                .map(|codelet_report| {
                    let status_label = codelet_report
                        .status
                        .as_ref()
                        .map(|s| s.label.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let is_skipped = codelet_report
                        .status
                        .as_ref()
                        .map(|s| format!("{:?}", s.status))
                        .unwrap_or_else(|| "False".to_string());

                    let status_style = if is_skipped == "True" {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    };

                    // For each `codelet_report`, create one row for each transition in statistics.
                    let transition = &codelet_report.statistics.transitions[Transition::Step];

                    Row::new(vec![
                        Cell::from(codelet_report.name.clone()),     // Inspector name
                        Cell::from(codelet_report.typename.clone()), // Inspector typename
                        Cell::from(status_label.clone()).style(status_style), // Status label
                        Cell::from(is_skipped.clone()),              // Skipped flag
                        Cell::from(format!("{}s", transition.duration.total().as_secs_f64())), // Total duration
                        Cell::from(transition.duration.count().to_string()), // Count
                        Cell::from(format!("{}s", transition.period.total().as_secs_f64())), // Period
                        Cell::from(transition.skipped_count.to_string()), // Skipped count
                    ])
                })
                .collect()
        },
    );

    // Create the combined table.
    let combined_table = Table::new(
        combined_rows,
        &[
            Constraint::Percentage(15), // Inspector name
            Constraint::Percentage(15), // Type name
            Constraint::Percentage(15), // Status label
            Constraint::Percentage(10), // Skipped flag
            Constraint::Percentage(15), // Total duration
            Constraint::Percentage(10), // Count
            Constraint::Percentage(10), // Period
            Constraint::Percentage(10), // Skipped count
        ],
    )
    .header(
        Row::new(vec![
            "Inspector",
            "Type",
            "Status",
            "Skipped",
            "Total Duration",
            "Count",
            "Period",
            "Skipped Count",
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Inspector Status & Transition Statistics"),
    );

    // Render the combined table.
    f.render_widget(combined_table, chunks[0]);
}
