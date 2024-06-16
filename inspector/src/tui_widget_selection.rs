// Copyright 2022 by David Weikersdorfer
use tui::widgets::{ListState, TableState};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CycleDirection {
    Next,
    Prev,
}

fn cycle(index: usize, delta: i32, max: usize) -> usize {
    if max == 0 {
        panic!("max must be positive");
    }
    if delta == 0 {
        if index >= max {
            max - 1
        } else {
            index
        }
    } else if delta == -1 {
        if index == 0 {
            max - 1
        } else if index - 1 < max {
            index - 1
        } else {
            max - 1
        }
    } else if delta == 1 {
        if index == usize::MAX || index + 1 >= max {
            0
        } else {
            index + 1
        }
    } else {
        panic!("delta must be -1, 0 or 1");
    }
}

pub trait TuiWdgtState {
    fn select(&mut self, a: Option<usize>);
    fn selected(&self) -> Option<usize>;
}

impl TuiWdgtState for TableState {
    fn select(&mut self, a: Option<usize>) {
        self.select(a)
    }
    fn selected(&self) -> Option<usize> {
        self.selected()
    }
}

impl TuiWdgtState for ListState {
    fn select(&mut self, a: Option<usize>) {
        self.select(a)
    }
    fn selected(&self) -> Option<usize> {
        self.selected()
    }
}

/// Stable selection of an item in a TUI widget based on a unique identifier
pub struct TuiWidgetSelection<T>
where
    T: Default + TuiWdgtState,
{
    state: T,
    uid: Option<u64>,
    items: Vec<u64>,
}

impl<T> Default for TuiWidgetSelection<T>
where
    T: Default + TuiWdgtState,
{
    fn default() -> Self {
        TuiWidgetSelection {
            state: T::default(),
            uid: None,
            items: vec![],
        }
    }
}

impl<T> TuiWidgetSelection<T>
where
    T: Default + TuiWdgtState,
{
    /// Updates the list of UIDs available for selection
    ///
    /// If the currently selected UID does not appear in the new list of items, the first of the
    /// new items is selected instead.
    pub fn set_items(&mut self, items: Vec<u64>) {
        self.items = items;
        self.delta(0);
    }

    /// Gets the list of items
    pub fn items(&self) -> &Vec<u64> {
        &self.items
    }

    /// UID of the selected item
    pub fn uid(&self) -> Option<u64> {
        self.uid
    }

    /// Index of the selected item based on the list of items
    pub fn index(&self) -> Option<usize> {
        if self.items.is_empty() || self.uid.is_none() {
            None
        } else {
            self.items.iter().position(|&h| h == self.uid.unwrap())
        }
    }

    /// The TUI widget state
    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    pub fn cycle(&mut self, dir: CycleDirection) {
        match dir {
            CycleDirection::Next => self.next(),
            CycleDirection::Prev => self.prev(),
        }
    }

    /// Selects the next item
    pub fn next(&mut self) {
        self.delta(1)
    }

    /// Selects the previous item
    pub fn prev(&mut self) {
        self.delta(-1)
    }

    fn delta(&mut self, delta: i32) {
        if self.items.is_empty() {
            self.state.select(None)
        } else {
            if let Some(index) = self.index() {
                self.state
                    .select(Some(cycle(index, delta, self.items.len())))
            } else {
                self.state.select(Some(0));
            }
        }
        if let Some(index) = self.state.selected() {
            self.uid = Some(self.items[index])
        }
    }
}
