// Copyright 2022 by David Weikersdorfer
use crate::node_ascii_art_tui_widget::NodeAsciiArtTuiWidget;
use crate::nodo::inspector as nodi;
use crate::tui_app_state::*;
use crate::tui_style::*;
use inspector_proto::lifecycle_state_to_str;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, List, ListItem, Paragraph, Row, Table, Widget},
    Frame,
};

fn strip_schema_and_server(uri: &str) -> &str {
    uri.strip_prefix("nodo:/")
        .map_or(uri, |x| if x.is_empty() { "/" } else { x })
}

pub fn widget_manifold<B>(
    frame: &mut Frame<B>,
    chunk: Rect,
    maybe_world: &Option<nodi::Worldstate>,
    style: &TuiStyle,
    state: &mut TuiAppState,
) where
    B: Backend,
{
    let sub_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(chunk);

    let vertices_hsv = maybe_world.as_ref().map_or(vec![], |w| w.vertices_hsv());

    state.set_vertex_choices(vertices_hsv.iter().map(|(h, _, _)| *h).collect());

    let vertex_uri_items: Vec<_> = vertices_hsv
        .iter()
        .map(|&(_, s, v)| {
            ListItem::new(Span::styled(
                strip_schema_and_server(s).to_string(),
                if v.is_executable {
                    if v.execution_data.as_ref().map_or(false, |x| x.has_failure) {
                        style.vertex_failure
                    } else {
                        style.vertex_active
                    }
                } else {
                    style.vertex_inactive
                },
            ))
        })
        .collect();

    let vertex_list = List::new(vertex_uri_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(style.section())
                .title(style.section_title("Vertices"))
                .border_type(BorderType::Plain),
        )
        .highlight_style(style.selection_highlight(state.arena() == SelectionArena::Vertex));

    frame.render_stateful_widget(
        vertex_list,
        sub_chunks[0],
        state.get_vertex_selection_mut().state_mut(),
    );

    if maybe_world.is_none() {
        return;
    }
    let world: &nodi::Worldstate = maybe_world.as_ref().unwrap();

    if let Some(vertex_sel_idx) = state.get_vertex_selection_mut().index() {
        let selected_vertex = vertices_hsv.get(vertex_sel_idx).unwrap().2;

        let vertex_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(10),
                    Constraint::Min(10),
                    Constraint::Min(10),
                    Constraint::Min(10),
                    Constraint::Min(10),
                    Constraint::Length(4),
                ]
                .as_ref(),
            )
            .split(sub_chunks[1]);

        let vertex_top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(vertex_chunks[0]);

        if let Some(exec_data) = selected_vertex.execution_data.as_ref() {
            frame.render_widget(
                Table::new(vec![
                    Row::new(vec![
                        Cell::from(Span::styled("status", style.default_text)),
                        Cell::from(if exec_data.has_failure {
                            Span::styled("FAIL", style.status_failure)
                        } else {
                            Span::styled("OK", style.status_ok)
                        }),
                    ]),
                    Row::new(vec![
                        Cell::from(Span::styled("current state", style.default_text)),
                        Cell::from(Span::styled(
                            lifecycle_state_to_str(exec_data.current_lifecycle_state),
                            style.default_text,
                        )),
                    ]),
                    Row::new(vec![
                        Cell::from(Span::styled("target state", style.default_text)),
                        Cell::from(Span::styled(
                            lifecycle_state_to_str(exec_data.target_lifecycle_state),
                            style.default_text,
                        )),
                    ]),
                    Row::new(vec![
                        Cell::from(Span::styled("scheduled", style.default_text)),
                        Cell::from(
                            style.yes_no(
                                selected_vertex
                                    .execution_data
                                    .as_ref()
                                    .map_or(false, |x| x.is_scheduled),
                                "yes",
                                "no",
                            ),
                        ),
                    ]),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(style.section())
                        .title(style.section_title("Execution"))
                        .border_type(BorderType::Plain),
                )
                .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)]),
                vertex_top_chunks[1],
            );
        }

        frame.render_widget(
            NodeAsciiArtTuiWidget::new(world, selected_vertex, style),
            vertex_top_chunks[0],
        );

        render_vertex_params(
            frame,
            vertex_chunks[1],
            world,
            style,
            state,
            selected_vertex,
        );

        render_vertex_channels(
            frame,
            vertex_chunks[2],
            style,
            state,
            world.vertex_rx_channels(selected_vertex),
            "RX Channels",
            SelectionArena::RxChannels,
        );

        render_vertex_channels(
            frame,
            vertex_chunks[3],
            style,
            state,
            world.vertex_tx_channels(selected_vertex),
            "TX Channels",
            SelectionArena::TxChannels,
        );

        frame.render_widget(
            widget_vertex_conditions(world, style, selected_vertex),
            vertex_chunks[4],
        );

        frame.render_widget(
            Paragraph::new(Span::styled(
                selected_vertex
                    .execution_data
                    .as_ref()
                    .map_or("", |x| &x.error_message),
                style.error_message,
            ))
            .block(
                Block::default()
                    .title(style.section_title("Status"))
                    .borders(Borders::ALL),
            )
            .style(style.section())
            .alignment(Alignment::Left),
            vertex_chunks[5],
        );
    }
}

fn fmt_tuid(tuid: &Option<nodi::Tuid>) -> String {
    tuid.as_ref()
        .map_or(String::from("error"), |x| format!("0x{:016X}", x.hash))
}

pub fn format_timestamp(ts: i64) -> String {
    let sec: i64 = ts / 1000000000;
    let ms: i64 = (ts - sec * 1000000000) / 1000000;
    format!("{}.{:03}", sec, ms)
}

fn channel_message_set<'a>(
    channel: &[(u64, String, &'a nodi::Channel)],
    idx: usize,
) -> Option<&'a nodi::MessageSet> {
    channel.get(idx)?.2.messages.as_ref()
}

fn channel_message_rows<'a>(
    channel: &[(u64, String, &nodi::Channel)],
    idx: usize,
    style: &TuiStyle,
) -> Option<Vec<Row<'a>>> {
    if let Some(msg_set) = channel_message_set(channel, idx) {
        let mut items = msg_set
            .item
            .iter()
            .map(|msg| {
                Row::new(vec![
                    Cell::from(style.default_text(format_timestamp(msg.pub_time))),
                    Cell::from(style.default_text(msg.pub_counter.to_string())),
                    Cell::from(style.default_text(format_timestamp(msg.acq_time))),
                    Cell::from(style.default_text(msg.acq_clock.to_string())),
                ])
            })
            .collect::<Vec<Row>>();
        if msg_set.num_skipped > 0 {
            let n = items.len() / 2; // len should be 6 ..
            items.insert(
                n,
                Row::new(vec![
                    Cell::from(style.default_text("...".to_string())),
                    Cell::from(style.default_text("...".to_string())),
                    Cell::from(style.default_text("...".to_string())),
                    Cell::from(style.default_text("...".to_string())),
                ]),
            );
        }
        Some(items)
    } else {
        None
    }
}

fn channels_rows<'a>(channels: &[(u64, String, &nodi::Channel)], style: &TuiStyle) -> Vec<Row<'a>> {
    channels
        .iter()
        .map(|(_, name, channel)| channel_row(name, channel, style))
        .collect::<Vec<Row>>()
}

fn channel_row<'a>(name: &str, channel: &nodi::Channel, style: &TuiStyle) -> Row<'a> {
    Row::new(vec![
        Cell::from(Span::styled(name.to_string(), style.default_text)),
        Cell::from(Span::styled(fmt_tuid(&channel.tuid), style.default_text)),
        Cell::from(if channel.is_alias {
            Span::styled("(alias)", style.default_text_low)
        } else {
            Span::styled(channel.cursor.to_string(), style.default_text)
        }),
        Cell::from(if channel.is_alias {
            Span::styled("(alias)", style.default_text_low)
        } else {
            style.available_count(channel.num_available)
        }),
    ])
}

fn render_vertex_channels<B>(
    frame: &mut Frame<B>,
    chunk: Rect,
    style: &TuiStyle,
    state: &mut TuiAppState,
    channels: Vec<(u64, String, &nodi::Channel)>,
    section_title: &str,
    selection_arena: SelectionArena,
) where
    B: Backend,
{
    let sub_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunk);

    let is_selected_arena = state.arena() == selection_arena;

    let sel = state.get_table_selection_mut(selection_arena);
    sel.set_items(channels.iter().map(|(h, _, _)| *h).collect());

    let contents_channels = Table::new(channels_rows(&channels, style))
        .header(Row::new(vec![
            Cell::from(Span::styled("NAME", style.table_header)),
            Cell::from(Span::styled("TUID", style.table_header)),
            Cell::from(Span::styled("CURSOR", style.table_header)),
            Cell::from(Span::styled("AVAILABLE", style.table_header)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(style.section())
                .title(style.section_title(section_title))
                .border_type(BorderType::Plain),
        )
        .highlight_style(if channels.len() > 1 {
            style.selection_highlight(is_selected_arena)
        } else {
            style.default_text
        })
        .widths(&[
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]);
    frame.render_stateful_widget(contents_channels, sub_chunks[0], sel.state_mut());

    let contents_messages = Table::new(sel.index().map_or(Vec::new(), |i| {
        channel_message_rows(&channels, i, style).unwrap_or_default()
    }))
    .header(Row::new(vec![
        Cell::from(Span::styled("PUB", style.table_header)),
        Cell::from(Span::styled("#", style.table_header)),
        Cell::from(Span::styled("ACQ", style.table_header)),
        Cell::from(Span::styled("CLK", style.table_header)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(style.section())
            .title(style.section_title("Channel Messages"))
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(30),
        Constraint::Percentage(20),
        Constraint::Percentage(30),
        Constraint::Percentage(20),
    ]);
    frame.render_widget(contents_messages, sub_chunks[1]);
}

fn condition_status_span<'a>(status_raw: i32, style: &TuiStyle) -> Span<'a> {
    match nodi::ConditionStatus::from_i32(status_raw) {
        Some(status) => match status {
            nodi::ConditionStatus::Invalid => Span::raw("(err)"),
            nodi::ConditionStatus::Satisfied => {
                Span::styled("Satisfied", style.condition_satisfied)
            }
            nodi::ConditionStatus::Waiting => Span::styled("Waiting", style.condition_waiting),
            nodi::ConditionStatus::Unsatisfied => {
                Span::styled("Unsatisfied", style.condition_unsatisfied)
            }
        },
        None => Span::styled("N/A", style.default_text),
    }
}

fn widget_vertex_conditions(
    world: &nodi::Worldstate,
    style: &TuiStyle,
    vertex: &nodi::Vertex,
) -> impl Widget {
    let conditions = world.vertex_conditions_individual(vertex);

    Table::new(
        conditions
            .iter()
            .map(|(_, name, condition)| {
                Row::new(vec![
                    Cell::from(Span::styled(name.clone(), style.default_text)),
                    Cell::from(condition_status_span(condition.status, style)),
                    Cell::from(Span::styled(
                        condition.deadline.to_string(),
                        style.default_text,
                    )),
                ])
            })
            .collect::<Vec<Row>>(),
    )
    .header(Row::new(vec![
        Cell::from(Span::styled("NAME", style.table_header)),
        Cell::from(Span::styled("STATUS", style.table_header)),
        Cell::from(Span::styled("DEADLINE", style.table_header)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(style.section())
            .title(style.section_title("Conditions"))
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(50),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
}

fn render_vertex_params<B>(
    frame: &mut Frame<B>,
    chunk: Rect,
    world: &nodi::Worldstate,
    style: &TuiStyle,
    state: &mut TuiAppState,
    vertex: &nodi::Vertex,
) where
    B: Backend,
{
    let sub_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunk);

    let is_selected_arena = state.arena() == SelectionArena::Parameter;

    let params = world.vertex_parameters(vertex);

    let psel = state.get_table_selection_mut(SelectionArena::Parameter);
    psel.set_items(params.iter().map(|(h, _, _)| *h).collect());

    let mut contents = Table::new(
        params
            .iter()
            .map(|(_, name, p)| {
                Row::new(vec![
                    Cell::from(Span::styled(name, style.default_text)),
                    Cell::from(Span::styled(fmt_tuid(&p.tuid), style.default_text)),
                    Cell::from(Span::styled(
                        if p.is_optional { "O" } else { "" }.to_owned()
                            + if p.is_mutable { "M" } else { "" },
                        style.default_text,
                    )),
                    Cell::from(if p.is_default {
                        Span::styled("D", style.parameter_default)
                    } else if p.is_alias && p.has_value {
                        Span::styled("Y*", style.parameter_alias_with_value)
                    } else if p.is_alias && !p.has_value {
                        Span::styled("N*", style.parameter_alias_no_value)
                    } else if p.has_value {
                        Span::styled("Y", style.parameter_with_value)
                    } else {
                        Span::styled("N", style.parameter_no_value)
                    }),
                ])
            })
            .collect::<Vec<Row>>(),
    )
    .header(Row::new(vec![
        Cell::from(Span::styled("NAME", style.table_header)),
        Cell::from(Span::styled("TUID", style.table_header)),
        Cell::from(Span::styled("FL", style.table_header)),
        Cell::from(Span::styled("OK", style.table_header)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(style.section())
            .title(style.section_title("Parameters"))
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(50),
        Constraint::Length(18),
        Constraint::Length(3),
        Constraint::Length(3),
    ]);
    if params.len() > 1 {
        contents = contents.highlight_style(style.selection_highlight(is_selected_arena));
    }
    frame.render_stateful_widget(contents, sub_chunks[0], psel.state_mut());

    let pinspect = Paragraph::new(psel.index().map_or("", |i| params[i].2.value.as_str()))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(style.section())
                .title(style.section_title("Parameter Value"))
                .border_type(BorderType::Plain),
        )
        .style(style.default_text);
    frame.render_widget(pinspect, sub_chunks[1]);
}
