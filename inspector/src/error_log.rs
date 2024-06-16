// Copyright 2022 by David Weikersdorfer
use std::time::Instant;

const RETENTION_DURATION_SEC: f64 = 3.0;

pub struct ErrorLog {
    messages: Vec<(f64, String)>,
    clock: Instant,
}

impl Default for ErrorLog {
    fn default() -> Self {
        ErrorLog {
            messages: vec![],
            clock: Instant::now(),
        }
    }
}

impl ErrorLog {
    pub fn push(&mut self, msg: String) {
        self.messages
            .push((self.clock.elapsed().as_secs_f64(), msg));
    }

    pub fn drain(&mut self) {
        let now = self.clock.elapsed().as_secs_f64();
        while !self.messages.is_empty() && self.messages[0].0 + RETENTION_DURATION_SEC > now {
            self.messages.drain(0..1);
        }
        if self.messages.len() > 8 {
            let n = self.messages.len() - 8;
            self.messages.drain(0..n);
        }
    }

    pub fn latest(&self) -> Option<&String> {
        Some(&self.messages.last()?.1)
    }
}
