use super::*;
use agent_desktop_core::{CursorAim, CursorMotionProfile};

fn button() -> Rect {
    Rect {
        x: 100.0,
        y: 200.0,
        width: 80.0,
        height: 30.0,
    }
}

fn state(spread: f64, drift: bool, at: Option<Point>, moves: u64) -> OverlayState {
    let mut aim = CursorAim::default();
    aim.set_spread(spread);
    aim.set_drift(drift);
    let mut motion = CursorMotionProfile::default();
    motion.set_aim(aim);
    motion.set_seed(Some(7));
    OverlayState {
        motion,
        at,
        moves,
        ..OverlayState::default()
    }
}

fn instruction(phase: CursorPhase, click: bool) -> CursorOverlayInstruction {
    let config = CursorOverlayConfig::enabled(None, 6).expect("valid config");
    CursorOverlayInstruction::new(Point { x: 140.0, y: 215.0 }, &config, click)
        .expect("valid instruction")
        .with_phase(phase)
        .with_target(Some(button()))
}

#[test]
fn travel_lands_inside_the_inner_area_and_varies_per_move() {
    let inner = inner(&button(), 0.5).expect("inner area");
    let mut landings = Vec::new();
    for moves in 0..40 {
        let aimed = aimed(
            &instruction(CursorPhase::Travel, false),
            &state(0.5, false, None, moves),
        );
        assert!(
            contains(&inner, aimed.destination()),
            "{:?}",
            aimed.destination()
        );
        assert!(aimed.target().is_none(), "travel never highlights");
        landings.push((aimed.destination().x, aimed.destination().y));
    }
    landings.sort_by(|a, b| a.partial_cmp(b).unwrap());
    landings.dedup();
    assert!(landings.len() > 30, "landings must vary between moves");
}

#[test]
fn a_cursor_already_inside_the_inner_area_stays_put_for_the_click() {
    let rest = Point { x: 132.0, y: 211.0 };
    let travel = aimed(
        &instruction(CursorPhase::Travel, true),
        &state(0.5, false, Some(rest.clone()), 3),
    );
    assert_eq!(travel.destination(), &rest);
    let effect = aimed(
        &instruction(CursorPhase::Effect, true),
        &state(0.5, false, Some(rest.clone()), 4),
    );
    assert_eq!(
        effect.destination(),
        &rest,
        "the ripple plays under the cursor"
    );
    assert_eq!(
        effect.target(),
        Some(&button()),
        "the effect keeps its highlight"
    );
}

#[test]
fn without_spread_travel_lands_on_the_centre() {
    let aimed = aimed(
        &instruction(CursorPhase::Travel, false),
        &state(0.0, true, None, 1),
    );
    assert_eq!(aimed.destination(), &Point { x: 140.0, y: 215.0 });
}

#[test]
fn the_inner_area_keeps_clear_of_a_tiny_controls_edges() {
    let tiny = Rect {
        x: 0.0,
        y: 0.0,
        width: 6.0,
        height: 40.0,
    };
    let inner = inner(&tiny, 1.0).expect("inner area");
    assert_eq!(inner.width, 0.0);
    assert_eq!(inner.height, 34.0);
}

#[test]
fn drift_is_planned_only_after_a_click_when_enabled() {
    let mut with_drift = state(0.5, true, None, 1);
    plan_drift(&instruction(CursorPhase::Effect, true), &mut with_drift);
    assert!(with_drift.drift.is_some());
    plan_drift(&instruction(CursorPhase::Effect, false), &mut with_drift);
    assert!(
        with_drift.drift.is_none(),
        "a hover or scroll does not drift"
    );
    let mut without = state(0.5, false, None, 1);
    plan_drift(&instruction(CursorPhase::Effect, true), &mut without);
    assert!(without.drift.is_none());
}

#[test]
fn drift_leaves_a_small_control_to_a_nearby_spot() {
    let screen = Rect {
        x: 0.0,
        y: 0.0,
        width: 1_500.0,
        height: 900.0,
    };
    let at = Point { x: 140.0, y: 215.0 };
    let mut spots = Vec::new();
    for moves in 0..40 {
        let point = drift_point(&button(), &at, &screen, &state(0.5, true, None, moves));
        assert!(
            !contains(&button(), &point),
            "{point:?} must be off the control"
        );
        let distance = ((point.x - at.x).powi(2) + (point.y - at.y).powi(2)).sqrt();
        assert!(distance < 120.0, "{point:?} must stay near the control");
        spots.push(point.x);
    }
    spots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    spots.dedup();
    assert!(spots.len() > 30, "drift spots must vary");
}
