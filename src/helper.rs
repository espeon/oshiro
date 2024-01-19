use chrono::DateTime;
use chrono::Utc;

pub struct Timer {
    start: DateTime<Utc>,
}

impl Default for Timer {
    fn default() -> Self {
        Timer { start: Utc::now() }
    }
}
impl Timer {

    pub fn new() -> Self {
        Default::default()
    }

    pub fn elapsed_ms(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.start)
            .num_milliseconds()
    }
}
