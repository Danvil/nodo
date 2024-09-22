// Copyright 2023 by David Weikersdorfer. All rights reserved.

use nodo_core::AcqtimeMarker;
use nodo_core::AppMonotonicClock;
use nodo_core::Clock;
use nodo_core::Pubtime;
use nodo_core::PubtimeMarker;
use nodo_core::SysMonotonicClock;

/// Task clocks used internally
#[derive(Clone)]
pub(crate) struct Clocks {
    /// Application-wide mononotic clock starting when the application starts
    pub app_mono: AppMonotonicClock<PubtimeMarker>,

    /// System-wide monotonic clock (probably) starting when the system boots
    pub sys_mono: SysMonotonicClock<AcqtimeMarker>,
}

impl Clocks {
    pub(crate) fn new() -> Self {
        Self {
            app_mono: AppMonotonicClock::new(),
            sys_mono: SysMonotonicClock::new(),
        }
    }
}

/// Clocks interface exposed to codelet
#[derive(Clone)]
pub struct TaskClocks {
    /// Application-wide mononotic clock starting when the application starts
    pub app_mono: AppMonotonicClock<PubtimeMarker>,

    /// System-wide monotonic clock (probably) starting when the system boots
    pub sys_mono: SysMonotonicClock<AcqtimeMarker>,

    /// Codelet-specific timings
    pub codelet: CodeletClock,

    pub(crate) deprecated_task_clock: TaskClock,
}

impl TaskClocks {
    pub(crate) fn from(clocks: Clocks) -> Self {
        Self {
            app_mono: clocks.app_mono.clone(),
            sys_mono: clocks.sys_mono.clone(),
            codelet: CodeletClock::new(clocks.app_mono.now()),
            deprecated_task_clock: TaskClock::from(clocks.app_mono.clone()),
        }
    }

    pub(crate) fn on_codelet_start(&mut self) {
        let now = self.app_mono.now();
        self.codelet.last = now;
        self.deprecated_task_clock.start(now);
    }

    pub(crate) fn on_codelet_stop(&mut self) {}

    pub(crate) fn on_codelet_step(&mut self) {
        let now = self.app_mono.now();
        self.codelet.update_dt(now);
        self.deprecated_task_clock.step(now);
    }
}

#[derive(Clone)]
pub struct CodeletClock {
    last: Pubtime,
    dt: f32,
}

impl CodeletClock {
    pub fn new(now: Pubtime) -> Self {
        Self { last: now, dt: 0.0 }
    }

    pub fn update_dt(&mut self, now: Pubtime) {
        let dt = self.last.abs_diff(now).as_secs_f32();
        self.last = now;
        self.dt = dt;
    }

    /// Time when the current step started. `step_time` is set at the beginning of start/step/stop
    /// functions and stays constant throughout the current step. Use `real_time` for a continuously
    /// updating time.
    pub fn step_time(&self) -> Pubtime {
        self.last
    }

    /// Time elapsed in seconds since last step.
    pub fn dt_secs_f32(&self) -> f32 {
        self.dt
    }
}

#[derive(Clone)]
pub struct TaskClock {
    clock: AppMonotonicClock<PubtimeMarker>,
    last: Pubtime,
    dt: f32,
}

impl TaskClock {
    pub fn from(clock: AppMonotonicClock<PubtimeMarker>) -> Self {
        let last = clock.now();
        Self {
            clock,
            last,
            dt: 0.0,
        }
    }

    pub(crate) fn start(&mut self, now: Pubtime) {
        self.last = now;
    }

    pub(crate) fn step(&mut self, now: Pubtime) {
        let dt = self.last.abs_diff(now).as_secs_f32();
        self.last = now;
        self.dt = dt;
    }

    /// Time when the current step started. `step_time` is set at the beginning of start/step/stop
    /// functions and stays constant throughout the current step. Use `real_time` for a continuously
    /// updating time.
    #[deprecated(note = "Use cx.clocks.codelet.step_time() instead")]
    pub fn step_time(&self) -> Pubtime {
        self.last
    }

    /// The current time from the default clock. `real_time` changes during start/step/stop
    /// functions. Use `step_time` for a timestep which remains constant during those functions.
    #[deprecated(note = "Use cx.clocks.app_mono.now() instead")]
    pub fn real_time(&self) -> Pubtime {
        self.clock.now()
    }

    /// Time elapsed in seconds since last step.
    // TODO Use Duration and introduce `dt_seconds_f32` (?).
    #[deprecated(note = "Use cx.clocks.codelet.dt() instead")]
    pub fn dt(&self) -> f32 {
        self.dt
    }
}
