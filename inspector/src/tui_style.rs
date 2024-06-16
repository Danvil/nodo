// Copyright 2022 by David Weikersdorfer
use tui::style::{Color, Modifier, Style};
use tui::text::Span;

#[derive(Debug)]
pub struct TuiStyle {
    pub default_text: Style,
    pub default_text_low: Style,
    section: Style,
    section_title: Style,
    pub menu_selection: Style,
    pub table_header: Style,
    pub selection_highlight_active: Style,
    pub selection_highlight_inactive: Style,
    pub vertex_inactive: Style,
    pub vertex_active: Style,
    pub vertex_failure: Style,
    pub condition_satisfied: Style,
    pub condition_waiting: Style,
    pub condition_unsatisfied: Style,
    pub available_none: Style,
    pub available_some: Style,
    pub ascii_art_node_channel_tag: Style,
    pub ascii_art_node_alias_channel_tag: Style,
    pub channel_empty: Style,
    pub channel_available: Style,
    pub channel_alias: Style,
    pub parameter_default: Style,
    pub parameter_alias_with_value: Style,
    pub parameter_alias_no_value: Style,
    pub parameter_with_value: Style,
    pub parameter_no_value: Style,
    pub status_failure: Style,
    pub status_ok: Style,
    pub bar: Style,
    pub error_message: Style,
}

impl TuiStyle {
    pub fn default() -> TuiStyle {
        const PASSIVE_COLOR: Color = Color::Cyan;
        const HIGHLIGHT_COLOR: Color = Color::LightCyan;

        TuiStyle {
            default_text: Style::default().fg(Color::White),
            default_text_low: Style::default().fg(Color::DarkGray),
            section: Style::default().fg(PASSIVE_COLOR),
            section_title: Style::default().fg(PASSIVE_COLOR),
            menu_selection: Style::default()
                .fg(HIGHLIGHT_COLOR)
                .add_modifier(Modifier::UNDERLINED),
            table_header: Style::default()
                .fg(PASSIVE_COLOR)
                .add_modifier(Modifier::UNDERLINED),
            selection_highlight_active: Style::default().bg(HIGHLIGHT_COLOR).fg(Color::Black),
            selection_highlight_inactive: Style::default().bg(Color::DarkGray).fg(Color::Black),
            vertex_inactive: Style::default().fg(Color::DarkGray),
            vertex_active: Style::default().fg(Color::White),
            vertex_failure: Style::default().fg(Color::LightRed),
            condition_satisfied: Style::default().fg(Color::LightGreen),
            condition_waiting: Style::default().fg(Color::LightYellow),
            condition_unsatisfied: Style::default().fg(Color::LightRed),
            available_none: Style::default().fg(Color::White),
            available_some: Style::default().fg(Color::Green),
            channel_empty: Style::default().fg(Color::White),
            channel_available: Style::default().fg(Color::Green),
            channel_alias: Style::default().fg(Color::DarkGray),
            parameter_default: Style::default().fg(Color::LightYellow),
            parameter_alias_with_value: Style::default().fg(Color::Green),
            parameter_alias_no_value: Style::default().fg(Color::Red),
            parameter_with_value: Style::default().fg(Color::LightGreen),
            parameter_no_value: Style::default().fg(Color::LightRed),
            ascii_art_node_channel_tag: Style::default().fg(Color::Black).bg(Color::White),
            ascii_art_node_alias_channel_tag: Style::default().fg(Color::Black).bg(Color::DarkGray),
            status_failure: Style::default().fg(Color::Black).bg(Color::Red),
            status_ok: Style::default().fg(Color::Black).bg(Color::Green),
            bar: Style::default().fg(PASSIVE_COLOR).bg(Color::DarkGray),
            error_message: Style::default().fg(Color::LightRed),
        }
    }

    pub fn selection_highlight(&self, active: bool) -> Style {
        if active {
            self.selection_highlight_active
        } else {
            self.selection_highlight_inactive
        }
    }

    pub fn default_text<'a>(&self, text: String) -> Span<'a> {
        Span::styled(text, self.default_text)
    }

    pub fn available_count<'a>(&self, n: i64) -> Span<'a> {
        Span::styled(
            n.to_string(),
            if n == 0 {
                self.available_none
            } else {
                self.available_some
            },
        )
    }

    pub fn section_title<'a>(&self, text: &'a str) -> Span<'a> {
        Span::styled(text, self.section_title)
    }

    pub fn section(&self) -> Style {
        self.section
    }

    pub fn yes_no_style(&self, flag: bool) -> Style {
        if flag {
            self.available_some
        } else {
            self.available_none
        }
    }

    pub fn yes_no<'a>(&self, flag: bool, yes_text: &'a str, no_text: &'a str) -> Span<'a> {
        Span::styled(
            if flag { yes_text } else { no_text },
            self.yes_no_style(flag),
        )
    }
}
