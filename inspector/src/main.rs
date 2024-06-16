// Copyright 2022 by David Weikersdorfer
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use inspector::nodo::inspector as nodi;
use inspector::nodo_app_link::*;
use inspector::tui_app_state::*;
use inspector::tui_style::*;
use inspector::widget_home::*;
use inspector::widget_manifold::*;
use inspector::widget_statistics::*;
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
    {thread, time},
};
use tui::{
    backend::Backend,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs, Widget},
    Frame, Terminal,
};

// const ADDRESS: &str = "tcp://192.168.8.228:12345";
const ADDRESS: &str = "tcp://localhost:12345";
const REDRAW_INTERVAL_MS: u64 = 50;
const DATA_POLL_INTERVAL_MS: u64 = 100;

enum Event<I> {
    Input(I),
    Tick,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    // thread to poll input from user
    let (user_event_tx, user_event_rx) = mpsc::channel();
    thread::spawn(move || {
        let tick_rate = Duration::from_millis(REDRAW_INTERVAL_MS);
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    user_event_tx
                        .send(Event::Input(key))
                        .expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if user_event_tx.send(Event::Tick).is_ok() {
                    last_tick = Instant::now();
                }
            }
        }
    });

    // thread to poll data from app
    let (link_tx, link_rx) = mpsc::channel();
    thread::spawn(move || {
        let interval = time::Duration::from_millis(DATA_POLL_INTERVAL_MS);
        let mut link = NodoAppLink::open(ADDRESS);
        loop {
            link_tx
                .send((link.request(), link.last_message_size))
                .unwrap();
            thread::sleep(interval);
        }
    });

    // Ctrl+C support
    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_requested_cpy = stop_requested.clone();
    ctrlc::set_handler(move || {
        println!("Stopping due to Ctrl+C!");
        stop_requested_cpy.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    // enter tui mode
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let style = TuiStyle::default();

    let mut state = TuiAppState::default();

    let mut last_world_state: Option<nodi::Worldstate> = None;

    while !stop_requested.load(Ordering::Relaxed) {
        // update manifold
        let maybe_ws = match link_rx.try_recv() {
            Ok(msg) => Some((msg.0.ok(), msg.1)),
            Err(e) => {
                state
                    .errors
                    .push(format!("Received invalid message from app: {:?}", e));
                None
            }
        };
        if let Some(ws) = maybe_ws {
            last_world_state = ws.0;
            state.message_size = ws.1;
        }

        state.errors.drain();

        // draw TUI
        terminal.draw(|frame| widget_main(frame, &last_world_state, &style, &mut state));

        // process user input
        match user_event_rx.recv()? {
            Event::Input(event) => state.process_key_code(event.code),
            Event::Tick => {}
        }

        if state.wants_to_stop() {
            break;
        }
    }

    // exit tui mode
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn widget_main<B>(
    frame: &mut Frame<B>,
    world: &Option<nodi::Worldstate>,
    style: &TuiStyle,
    state: &mut TuiAppState,
) where
    B: Backend,
{
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(frame.size());

    frame.render_widget(widget_menu(state, style), rects[0]);
    match state.active_menu_item {
        MenuItem::Home => frame.render_widget(widget_home(style), rects[1]),
        MenuItem::Manifold => widget_manifold(frame, rects[1], world, style, state),
        MenuItem::Statistics => widget_statistics(frame, rects[1], world, style, state),
    }
    frame.render_widget(widget_footer(world, state, style), rects[2]);
}

fn widget_menu(state: &mut TuiAppState, style: &TuiStyle) -> impl Widget {
    let menu_titles = vec!["Home", "Manifold", "Statistics", "Quit"];
    let menu = menu_titles
        .iter()
        .map(|t| {
            let (first, rest) = t.split_at(1);
            Spans::from(vec![
                Span::styled(first, style.menu_selection),
                Span::styled(rest, style.default_text),
            ])
        })
        .collect();

    Tabs::new(menu)
        .select(state.active_menu_item.into())
        .block(
            Block::default()
                .title(style.section_title(" NODOG "))
                .borders(Borders::ALL),
        )
        .style(style.section())
        .highlight_style(style.menu_selection)
        .divider(Span::raw("~"))
}

fn widget_footer(
    world: &Option<nodi::Worldstate>,
    state: &TuiAppState,
    style: &TuiStyle,
) -> impl Widget {
    Paragraph::new(Text::from(format!(
        "Msg Size: {} kB, App Time: {}, System Time: {}, Error: {}",
        state.message_size / 1000,
        world
            .as_ref()
            .map_or(String::from("N/A"), |w| format_timestamp(w.app_time)),
        world
            .as_ref()
            .map_or(String::from("N/A"), |w| format_timestamp(w.system_time)),
        state.errors.latest().map_or("", |x| x.as_str())
    )))
    .style(style.error_message)
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::NONE)
            .style(style.section())
            .border_type(BorderType::Plain),
    )
}
