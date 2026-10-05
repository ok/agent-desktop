use agent_desktop_core::{
    AdapterError, CursorOverlayConfig, CursorOverlayInstruction, CursorPhase, Point, Rect,
};
use std::f64::consts::{FRAC_PI_4, PI};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::{OverlayState, bridge, render::render};

const EDGE_INSET: f64 = 3.0;
const DRIFT_DELAY_MS: (f64, f64) = (420.0, 900.0);
const EXIT_MARGIN: (f64, f64) = (10.0, 34.0);
const SHORT_DRIFT: (f64, f64) = (22.0, 46.0);
const SMALL_CONTROL: f64 = 140.0;
const SCREEN_INSET: f64 = 8.0;
const DRIFT_FAN: f64 = 1.4 * PI;

/// A pending move off a clicked control, due once the click has settled.
pub(super) struct Drift {
    due: Instant,
    target: Rect,
    label: Option<String>,
}

/// The instruction as drawn. Travel lands inside the control's inner area, or stays where the
/// cursor already rests inside it, and never highlights; an effect plays where the cursor rests
/// when that is on the control.
pub(super) fn aimed(
    instruction: &CursorOverlayInstruction,
    state: &OverlayState,
) -> CursorOverlayInstruction {
    let aim = state.motion.aim();
    let at = state.at.as_ref();
    match (instruction.phase(), instruction.target()) {
        (CursorPhase::Travel, Some(target)) => {
            let travel = instruction.clone().with_target(None);
            let Some(inner) = inner(target, aim.spread()) else {
                return travel;
            };
            match at {
                Some(at) if contains(&inner, at) => travel.with_destination(at.clone()),
                _ => travel.with_destination(within(&inner, state)),
            }
        }
        (CursorPhase::Effect, Some(target)) if !aim.is_default() => match at {
            Some(at) if contains(target, at) => instruction.clone().with_destination(at.clone()),
            _ => instruction.clone(),
        },
        _ => instruction.clone(),
    }
}

/// After a click, plans a move off the control unless another action arrives first.
pub(super) fn plan_drift(instruction: &CursorOverlayInstruction, state: &mut OverlayState) {
    state.drift = match instruction.target() {
        Some(target)
            if state.motion.aim().drift()
                && instruction.phase() == CursorPhase::Effect
                && instruction.is_click() =>
        {
            let delay = lerp(DRIFT_DELAY_MS, draw(state, 7));
            Some(Drift {
                due: Instant::now() + Duration::from_millis(delay as u64),
                target: *target,
                label: instruction.label().map(str::to_owned),
            })
        }
        _ => None,
    };
}

/// Moves off the clicked control once the planned drift is due.
pub(super) fn drift_when_due(state: &mut OverlayState) -> Result<(), AdapterError> {
    let Some(drift) = state.drift.take_if(|drift| Instant::now() >= drift.due) else {
        return Ok(());
    };
    let Some(at) = state.at.clone() else {
        return Ok(());
    };
    let (screen, _, _) = bridge::screen_at(&at)?;
    let point = drift_point(&drift.target, &at, &screen, state);
    let config = CursorOverlayConfig::enabled(drift.label, 12)?;
    render(
        &CursorOverlayInstruction::new(point.clone(), &config, false)?,
        state,
    )?;
    state.at = Some(point);
    state.moves = state.moves.wrapping_add(1);
    state.settle.schedule(Instant::now());
    Ok(())
}

/// Mostly down and to the right, as a hand settles: off a small control by a margin past its
/// edge, a short way across a large one.
fn drift_point(target: &Rect, at: &Point, screen: &Rect, state: &OverlayState) -> Point {
    let angle = FRAC_PI_4 + (draw(state, 5) - 0.5) * DRIFT_FAN;
    let direction = (angle.cos(), angle.sin());
    let distance = if target.width.max(target.height) <= SMALL_CONTROL {
        exit_distance(target, at, direction) + lerp(EXIT_MARGIN, draw(state, 6))
    } else {
        lerp(SHORT_DRIFT, draw(state, 6))
    };
    Point {
        x: (at.x + direction.0 * distance).clamp(
            screen.x + SCREEN_INSET,
            screen.x + screen.width - SCREEN_INSET,
        ),
        y: (at.y + direction.1 * distance).clamp(
            screen.y + SCREEN_INSET,
            screen.y + screen.height - SCREEN_INSET,
        ),
    }
}

fn exit_distance(target: &Rect, at: &Point, direction: (f64, f64)) -> f64 {
    let along = |from: f64, low: f64, high: f64, step: f64| {
        if step > f64::EPSILON {
            (high - from) / step
        } else if step < -f64::EPSILON {
            (low - from) / step
        } else {
            f64::INFINITY
        }
    };
    along(at.x, target.x, target.x + target.width, direction.0)
        .min(along(at.y, target.y, target.y + target.height, direction.1))
        .max(0.0)
}

fn inner(target: &Rect, spread: f64) -> Option<Rect> {
    if spread <= 0.0 {
        return None;
    }
    let half_width = (target.width * 0.5 * spread).min((target.width * 0.5 - EDGE_INSET).max(0.0));
    let half_height =
        (target.height * 0.5 * spread).min((target.height * 0.5 - EDGE_INSET).max(0.0));
    Some(Rect {
        x: target.x + target.width * 0.5 - half_width,
        y: target.y + target.height * 0.5 - half_height,
        width: half_width * 2.0,
        height: half_height * 2.0,
    })
}

/// A point in `inner`, weighted towards its centre: each axis sums two uniform draws.
fn within(inner: &Rect, state: &OverlayState) -> Point {
    let offset = |a: u64, b: u64| (draw(state, a) + draw(state, b)) * 0.5;
    Point {
        x: inner.x + inner.width * offset(1, 2),
        y: inner.y + inner.height * offset(3, 4),
    }
}

fn contains(rect: &Rect, point: &Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn lerp(range: (f64, f64), t: f64) -> f64 {
    range.0 + (range.1 - range.0) * t
}

/// A draw in [0, 1) for this move and `salt`: reproducible from the motion seed, else varied
/// per renderer.
fn draw(state: &OverlayState, salt: u64) -> f64 {
    static ENTROPY: OnceLock<u64> = OnceLock::new();
    let seed = state.motion.seed().unwrap_or_else(|| {
        *ENTROPY.get_or_init(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos() as u64)
                ^ u64::from(std::process::id())
        })
    });
    let mut z = seed
        ^ state.moves.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1_u64 << 53) as f64
}

#[cfg(test)]
#[path = "child_aim_tests.rs"]
mod tests;
