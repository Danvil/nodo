// Copyright 2022 by David Weikersdorfer
use crate::nodo::inspector as nodi;
use crate::tui_app_state::TuiAppState;
use crate::tui_style::TuiStyle;
use tui::widgets::BarChart;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
    Frame,
};

fn stats_count_cell<'a, F>(
    stats: Option<&nodi::SignalStatistics>,
    style: &TuiStyle,
    f: F,
    width: usize,
) -> Cell<'a>
where
    F: Fn(&nodi::SignalStatistics) -> i64,
{
    Cell::from(
        stats
            .as_ref()
            .map_or(Span::styled("err", style.status_failure), |x| {
                let n = f(x);
                Span::styled(
                    format!("{1:>0$}", width, n),
                    if n == 0 {
                        style.default_text_low
                    } else {
                        style.default_text
                    },
                )
            }),
    )
}

fn stats_dt_cell<'a, F>(stats: Option<&nodi::SignalStatistics>, style: &TuiStyle, f: F) -> Cell<'a>
where
    F: Fn(&nodi::SignalStatistics) -> (i64, f64),
{
    Cell::from(
        stats
            .as_ref()
            .map_or(Span::styled("err", style.status_failure), |x| {
                let (n, dt) = f(x);
                if n == 0 {
                    Span::styled("    n/a", style.default_text_low)
                } else {
                    Span::styled(format!("{:7.4}", dt / 1000000000.0), style.default_text)
                }
            }),
    )
}

const LEN_COUNT_TICK: usize = 9;
const LEN_COUNT_OTHER: usize = 7;
const LEN_COUNT_SHORT: usize = 2;
const LEN_DELTAT: usize = 7;

fn statistics_row<'a>(style: &TuiStyle, name: &'a str, v: &nodi::Vertex) -> Option<Row<'a>> {
    if let Some(stats) = v.statistics.as_ref() {
        let on_tick_stats = stats.on_tick.as_ref();
        let on_start_stats = stats.on_start.as_ref();
        let on_stop_stats = stats.on_stop.as_ref();
        let on_pause_stats = stats.on_pause.as_ref();
        let on_resume_stats = stats.on_resume.as_ref();
        let on_create_stats = stats.on_create.as_ref();
        let on_destroy_stats = stats.on_destroy.as_ref();
        Some(Row::new(vec![
            Cell::from(Span::styled(name, style.default_text)),
            stats_count_cell(on_tick_stats, style, |s| s.count, LEN_COUNT_TICK),
            stats_dt_cell(on_tick_stats, style, |s| (s.count, s.average_interval)),
            stats_dt_cell(on_tick_stats, style, |s| (s.count, s.average_duration)),
            stats_count_cell(on_start_stats, style, |s| s.count, LEN_COUNT_OTHER),
            stats_dt_cell(on_start_stats, style, |s| (s.count, s.average_duration)),
            stats_count_cell(on_stop_stats, style, |s| s.count, LEN_COUNT_OTHER),
            stats_dt_cell(on_stop_stats, style, |s| (s.count, s.average_duration)),
            stats_count_cell(on_pause_stats, style, |s| s.count, LEN_COUNT_OTHER),
            stats_dt_cell(on_pause_stats, style, |s| (s.count, s.average_duration)),
            stats_count_cell(on_resume_stats, style, |s| s.count, LEN_COUNT_OTHER),
            stats_dt_cell(on_resume_stats, style, |s| (s.count, s.average_duration)),
            stats_count_cell(on_create_stats, style, |s| s.count, LEN_COUNT_SHORT),
            stats_count_cell(on_destroy_stats, style, |s| s.count, LEN_COUNT_SHORT),
        ]))
    } else {
        None
    }
}

pub fn widget_statistics<B>(
    frame: &mut Frame<B>,
    chunk: Rect,
    maybe_world: &Option<nodi::Worldstate>,
    style: &TuiStyle,
    state: &mut TuiAppState,
) where
    B: Backend,
{
    let sub_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(chunk);

    let vertices_hsv = maybe_world.as_ref().map_or(vec![], |w| w.vertices_hsv());

    frame.render_widget(
        Table::new(
            vertices_hsv
                .iter()
                .filter_map(|(_, s, v)| statistics_row(style, s, v)),
        )
        .header(Row::new(vec![
            Cell::from(Span::styled("NAME", style.table_header)),
            Cell::from(Span::styled("   TICK #", style.table_header)),
            Cell::from(Span::styled(" TICK I", style.table_header)),
            Cell::from(Span::styled(" TICK D", style.table_header)),
            Cell::from(Span::styled("START #", style.table_header)),
            Cell::from(Span::styled("START D", style.table_header)),
            Cell::from(Span::styled(" STOP #", style.table_header)),
            Cell::from(Span::styled(" STOP D", style.table_header)),
            Cell::from(Span::styled("PAUSE #", style.table_header)),
            Cell::from(Span::styled("PAUSE D", style.table_header)),
            Cell::from(Span::styled("RESME #", style.table_header)),
            Cell::from(Span::styled("RESME D", style.table_header)),
            Cell::from(Span::styled("+#", style.table_header)),
            Cell::from(Span::styled("-#", style.table_header)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(style.section())
                .title(style.section_title("Statistics"))
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(100),
            Constraint::Length(LEN_COUNT_TICK as u16),
            Constraint::Length(LEN_DELTAT as u16),
            Constraint::Length(LEN_DELTAT as u16),
            Constraint::Length(LEN_COUNT_OTHER as u16),
            Constraint::Length(LEN_DELTAT as u16),
            Constraint::Length(LEN_COUNT_OTHER as u16),
            Constraint::Length(LEN_DELTAT as u16),
            Constraint::Length(LEN_COUNT_OTHER as u16),
            Constraint::Length(LEN_DELTAT as u16),
            Constraint::Length(LEN_COUNT_OTHER as u16),
            Constraint::Length(LEN_DELTAT as u16),
            Constraint::Length(2),
            Constraint::Length(2),
        ]),
        sub_chunks[0],
    );

    if maybe_world.is_none() {
        return;
    }
    let workers = &maybe_world.as_ref().unwrap().workers;
    if !workers.is_empty() {
        let bar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(4); workers.len()])
            .split(sub_chunks[1]);
        for i in 0..workers.len() {
            if let Some(data) = (|| {
                Some(worker_load(
                    &workers[i].statistics.as_ref()?.entries,
                    maybe_world.as_ref()?.app_time,
                ))
            })() {
                // print!("{:?}", data);
                let bars = BarChart::default()
                    // .block(Block::default().title("BarChart").borders(Borders::ALL))
                    .bar_width(2)
                    .bar_gap(0)
                    .bar_style(style.bar)
                    // .value_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                    // .label_style(Style::default().fg(Color::White))
                    .data(&data)
                    .max(1000);
                frame.render_widget(bars, bar_chunks[i]);
            }
        }
    }
}

fn worker_load<'a>(stats: &Vec<nodi::WorkerStatisticsEntry>, now: i64) -> Vec<(&'a str, u64)> {
    const DT: i64 = 32 * 1000000;
    const N: i64 = 32;
    let start = ((now / DT) - N) * DT;
    let mut data = vec![("", 0_u64); N as usize];
    for s in stats {
        let b1: usize = ((s.begin - start) / DT).clamp(0, N - 1) as usize;
        let b2: usize = ((s.end - start) / DT).clamp(0, N - 1) as usize;
        for i in b1..b2 {
            data[i].1 = 1000;
        }
        if (b2 as i64) * DT < s.end && s.end < ((b2 + 1) as i64) * DT {
            data[b2].1 = ((1000 * (s.end - (b2 as i64) * DT)) / DT) as u64;
        }
    }
    data
}
