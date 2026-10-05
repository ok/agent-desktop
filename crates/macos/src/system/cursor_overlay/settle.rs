use agent_desktop_core::{CursorOverlayStyle, CursorShape, Point};
use std::time::{Duration, Instant};

const CHECKS_AFTER_MS: [u64; 3] = [350, 1_000, 2_500];
const HIT_TEST_BUDGET: Duration = Duration::from_millis(250);

/// Re-reads what lies under the resting cursor after each move and effect, so its image follows
/// the screen as the system cursor's does: the pointer stays while the cursor rests on a
/// button, and gives way to the arrow when a click opens a dialog under it. The first check
/// follows the ripple; the later ones catch views that appear more slowly.
#[derive(Default)]
pub(super) struct ShapeSettle {
    since: Option<Instant>,
    next: usize,
}

impl ShapeSettle {
    pub(super) fn schedule(&mut self, now: Instant) {
        self.since = Some(now);
        self.next = 0;
    }

    pub(super) fn cancel(&mut self) {
        self.since = None;
    }

    /// Whether a check falls due at `now`; each scheduled check is reported once.
    pub(super) fn due(&mut self, now: Instant) -> bool {
        let Some(since) = self.since else {
            return false;
        };
        let Some(after) = CHECKS_AFTER_MS.get(self.next) else {
            self.since = None;
            return false;
        };
        if now.duration_since(since) < Duration::from_millis(*after) {
            return false;
        }
        self.next += 1;
        true
    }
}

/// Whether any image differs from the arrow, so a shape check can change what is drawn.
pub(super) fn has_shapes(style: &CursorOverlayStyle) -> bool {
    style.draws(CursorShape::Pointer) || style.draws(CursorShape::Text)
}

pub(super) fn apply(at: &Point) {
    if let Some(shape) = crate::tree::cursor_shape::shape_at(at, Instant::now() + HIT_TEST_BUDGET) {
        super::bridge::select_shape(shape);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_check_falls_due_once_in_order() {
        let start = Instant::now();
        let mut settle = ShapeSettle::default();
        assert!(!settle.due(start + Duration::from_secs(5)));
        settle.schedule(start);
        assert!(!settle.due(start + Duration::from_millis(100)));
        assert!(settle.due(start + Duration::from_millis(400)));
        assert!(!settle.due(start + Duration::from_millis(500)));
        assert!(settle.due(start + Duration::from_millis(1_100)));
        assert!(settle.due(start + Duration::from_millis(2_600)));
        assert!(!settle.due(start + Duration::from_secs(10)));
    }

    #[test]
    fn a_new_action_restarts_the_checks_and_cancel_stops_them() {
        let start = Instant::now();
        let mut settle = ShapeSettle::default();
        settle.schedule(start);
        assert!(settle.due(start + Duration::from_millis(400)));
        settle.schedule(start + Duration::from_millis(500));
        assert!(!settle.due(start + Duration::from_millis(600)));
        assert!(settle.due(start + Duration::from_millis(900)));
        settle.cancel();
        assert!(!settle.due(start + Duration::from_secs(10)));
    }
}
