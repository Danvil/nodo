// Copyright 2022 by David Weikersdorfer
use crate::error_log::*;
use crate::tui_widget_selection::*;
use crossterm::event::KeyCode;
use std::collections::HashMap;
use tui::widgets::{ListState, TableState};

#[derive(Copy, Clone, Debug)]
pub enum MenuItem {
    Home,
    Manifold,
    Statistics,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Manifold => 1,
            MenuItem::Statistics => 2,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SelectionArena {
    Vertex,
    RxChannels,
    TxChannels,
    Parameter,
}

pub struct TuiAppState {
    pub errors: ErrorLog,
    pub active_menu_item: MenuItem,
    arena: SelectionArena,
    vertex_selection: TuiWidgetSelection<ListState>,
    rx_channel_selection: HashMap<u64, TuiWidgetSelection<TableState>>,
    tx_channel_selection: HashMap<u64, TuiWidgetSelection<TableState>>,
    parameter_selection: HashMap<u64, TuiWidgetSelection<TableState>>,
    wants_to_stop: bool,
    pub message_size: usize,
}

impl Default for TuiAppState {
    fn default() -> Self {
        TuiAppState {
            errors: ErrorLog::default(),
            active_menu_item: MenuItem::Manifold,
            arena: SelectionArena::Vertex,
            vertex_selection: TuiWidgetSelection::default(),
            rx_channel_selection: HashMap::new(),
            tx_channel_selection: HashMap::new(),
            parameter_selection: HashMap::new(),
            wants_to_stop: false,
            message_size: 0,
        }
    }
}

impl TuiAppState {
    pub fn arena(&self) -> SelectionArena {
        self.arena
    }

    /// Cycle through arenas
    fn cycle_selection_arena_raw(&mut self, dir: CycleDirection) {
        use SelectionArena::*;
        self.arena = match dir {
            CycleDirection::Next => match self.arena {
                Vertex => RxChannels,
                RxChannels => TxChannels,
                TxChannels => Parameter,
                Parameter => Vertex,
            },
            CycleDirection::Prev => match self.arena {
                Vertex => Parameter,
                RxChannels => Vertex,
                TxChannels => RxChannels,
                Parameter => TxChannels,
            },
        }
    }

    /// Cycle through non-empty arenas, but always stop at Vertex arena
    fn cycle_selection_arena(&mut self, dir: CycleDirection) {
        if let Some(uid) = self.vertex_selection.uid() {
            loop {
                self.cycle_selection_arena_raw(dir);
                use SelectionArena::*;
                let sel = match self.arena {
                    Vertex => return,
                    RxChannels => &self.rx_channel_selection,
                    TxChannels => &self.tx_channel_selection,
                    Parameter => &self.parameter_selection,
                };
                if sel
                    .get(&uid)
                    .expect("selected vertex not present in arena")
                    .items()
                    .len()
                    > 1
                {
                    return;
                }
            }
        } else {
            self.arena = SelectionArena::Vertex;
        }
    }

    fn cycle_selection_arena_contents(&mut self, dir: CycleDirection) {
        use SelectionArena::*;
        match self.arena {
            Vertex => self.vertex_selection.cycle(dir),
            RxChannels => {
                if let Some(uid) = self.vertex_selection.uid() {
                    if let Some(selection) = self.rx_channel_selection.get_mut(&uid) {
                        selection.cycle(dir);
                    }
                }
            }
            TxChannels => {
                if let Some(uid) = self.vertex_selection.uid() {
                    if let Some(selection) = self.tx_channel_selection.get_mut(&uid) {
                        selection.cycle(dir);
                    }
                }
            }
            Parameter => {
                if let Some(uid) = self.vertex_selection.uid() {
                    if let Some(selection) = self.parameter_selection.get_mut(&uid) {
                        selection.cycle(dir);
                    }
                }
            }
        }
    }

    pub fn wants_to_stop(&self) -> bool {
        self.wants_to_stop
    }

    pub fn process_key_code(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => self.wants_to_stop = true,
            KeyCode::Char('h') => self.active_menu_item = MenuItem::Home,
            KeyCode::Char('m') => self.active_menu_item = MenuItem::Manifold,
            KeyCode::Char('s') => self.active_menu_item = MenuItem::Statistics,
            KeyCode::Down => self.cycle_selection_arena_contents(CycleDirection::Next),
            KeyCode::Up => self.cycle_selection_arena_contents(CycleDirection::Prev),
            KeyCode::Right | KeyCode::Tab => self.cycle_selection_arena(CycleDirection::Next),
            KeyCode::Left => self.cycle_selection_arena(CycleDirection::Prev),
            _ => {}
        }
    }

    pub fn set_vertex_choices(&mut self, vertex_choices: Vec<u64>) {
        self.vertex_selection.set_items(vertex_choices);
        for h in self.vertex_selection.items() {
            if !self.rx_channel_selection.contains_key(h) {
                self.rx_channel_selection
                    .insert(*h, TuiWidgetSelection::default());
            }
            if !self.tx_channel_selection.contains_key(h) {
                self.tx_channel_selection
                    .insert(*h, TuiWidgetSelection::default());
            }
            if !self.parameter_selection.contains_key(h) {
                self.parameter_selection
                    .insert(*h, TuiWidgetSelection::default());
            }
        }
    }

    pub fn get_vertex_selection_mut(&mut self) -> &mut TuiWidgetSelection<ListState> {
        &mut self.vertex_selection
    }

    pub fn get_table_selection_mut(
        &mut self,
        arena: SelectionArena,
    ) -> &mut TuiWidgetSelection<TableState> {
        match arena {
            SelectionArena::RxChannels => self
                .rx_channel_selection
                .get_mut(&self.vertex_selection.uid().unwrap())
                .unwrap(),
            SelectionArena::TxChannels => self
                .tx_channel_selection
                .get_mut(&self.vertex_selection.uid().unwrap())
                .unwrap(),
            SelectionArena::Parameter => self
                .parameter_selection
                .get_mut(&self.vertex_selection.uid().unwrap())
                .unwrap(),
            _ => panic!(),
        }
    }
}
