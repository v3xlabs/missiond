use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct ChromeState {
    pub current_playlist_id: Option<String>,
    pub current_tab_id: Option<String>,
    pub is_running: bool,
    /// The interval of the running rotation, or nothing while rotation is stopped.
    pub rotation: Option<Duration>,
    pub current_tab_opened_at: Option<SystemTime>,
    pub hold_until: Option<Instant>,
    /// What the playlist was showing before an alert took the screen, so ending the takeover
    /// returns to it rather than to wherever rotation happens to have reached.
    pub interrupted_tab_id: Option<String>,
}

impl ChromeState {
    pub const fn auto_rotate(&self) -> bool {
        self.rotation.is_some()
    }

    /// When the rotation task will next step, as Unix seconds.
    ///
    /// The task wakes on every wall clock multiple of the interval and skips a wake that falls
    /// inside a hold, so the answer is the first such multiple after now that the hold has also
    /// reached.
    pub fn next_rotation_at(&self) -> Option<u64> {
        let interval = self.rotation?.as_secs();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
        // Rounded up from the exact end: a hold ending a fraction past a boundary still holds
        // that boundary.
        let hold_until = self.hold_until.map(|until| {
            (now + until.saturating_duration_since(Instant::now()))
                .as_secs_f64()
                .ceil() as u64
        });

        next_boundary(interval, now.as_secs(), hold_until)
    }
}

fn next_boundary(interval: u64, now: u64, hold_until: Option<u64>) -> Option<u64> {
    if interval == 0 {
        return None;
    }

    let after_now = (now / interval + 1) * interval;
    let after_hold = hold_until.map_or(0, |until| until.div_ceil(interval) * interval);

    Some(after_now.max(after_hold))
}

#[cfg(test)]
mod tests {
    use super::next_boundary;

    #[test]
    fn the_next_step_is_the_next_multiple_of_the_interval() {
        assert_eq!(next_boundary(60, 1_000, None), Some(1_020));
        assert_eq!(next_boundary(60, 1_020, None), Some(1_080));
    }

    #[test]
    fn a_hold_skips_every_step_before_it_ends() {
        assert_eq!(next_boundary(60, 1_000, Some(1_100)), Some(1_140));
        assert_eq!(next_boundary(60, 1_000, Some(1_140)), Some(1_140));
    }

    #[test]
    fn an_expired_hold_changes_nothing() {
        assert_eq!(next_boundary(60, 1_000, Some(900)), Some(1_020));
    }

    #[test]
    fn a_sub_second_interval_has_no_boundary() {
        assert_eq!(next_boundary(0, 1_000, None), None);
    }
}
