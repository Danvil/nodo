// Copyright 2022 by David Weikersdorfer
use crate::tui_style::TuiStyle;
use tui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub fn widget_home(style: &TuiStyle) -> impl Widget {
    Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "NODO Inspector",
            Style::default().fg(Color::Red),
        )]),
        Spans::from(vec![Span::raw("")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(style.section())
            .title(style.section_title("Home"))
            .border_type(BorderType::Plain),
    )
}
