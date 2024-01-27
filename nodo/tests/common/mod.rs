// Copyright 2023 by David Weikersdorfer. All rights reserved.

pub struct CompleteGuard {
    completed: bool,
}

impl CompleteGuard {
    pub fn new() -> Self {
        Self { completed: false }
    }

    pub fn complete(&mut self) {
        self.completed = true;
    }
}

impl Drop for CompleteGuard {
    fn drop(&mut self) {
        assert!(self.completed, "not completed");
    }
}
