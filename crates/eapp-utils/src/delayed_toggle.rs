#[derive(Default, Clone, Copy)]
pub struct DelayedToggle {
    active: bool,
    last_toggle_time: f64,
}

impl DelayedToggle {
    pub fn new(initial: bool) -> Self {
        Self {
            active: initial,
            last_toggle_time: 0.0,
        }
    }

    pub fn active(&mut self, current_time: f64) {
        self.active = true;
        self.last_toggle_time = current_time;
    }

    pub fn update(&mut self, current_time: f64, delayed: f64) {
        if self.last_toggle_time + delayed <= current_time {
            self.active = false;
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
