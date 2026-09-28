/// How long a platform renderer may take to report that the cursor reached its
/// destination before the action dispatches anyway.
pub const CURSOR_ARRIVAL_TIMEOUT_MS: u64 = 900;

/// The largest cursor travel plus dwell that still leaves room for the socket
/// round trip and frame pacing inside the arrival timeout.
pub const CURSOR_MOTION_BUDGET_MS: u64 = CURSOR_ARRIVAL_TIMEOUT_MS - 200;

/// How long the overlay stays visible with no instruction before it rests.
pub const CURSOR_IDLE_REST_MS: u64 = 6_000;

/// How long the outline around a clicked element stays on screen.
pub const CURSOR_HIGHLIGHT_HOLD_MS: u64 = 900;
