use agent_desktop_core::{
    AdapterError, CursorMotion, CursorOverlayInstruction, CursorPhase, CursorPose, Point,
    place_label,
};

use super::{BUBBLE_SIZE, OverlayState, bridge};

pub(super) fn render(
    instruction: &CursorOverlayInstruction,
    state: &OverlayState,
) -> Result<(), AdapterError> {
    if instruction.phase() == CursorPhase::Drag {
        let from = instruction.drag_from().unwrap_or(instruction.destination());
        let (screen, fps, reduce_motion) = bridge::screen_at(from)?;
        let bubble = place_label(from, BUBBLE_SIZE, &screen);
        bridge::run(
            &[CursorPose::still(from.clone())],
            fps,
            instruction,
            reduce_motion,
            &bubble,
        )?;
        bridge::begin_drag(from, state.style.ripple() && !reduce_motion);
        return Ok(());
    }
    if instruction.phase() == CursorPhase::Effect {
        bridge::end_drag(instruction.destination(), instruction.drag_from().is_some());
    }
    let (screen, fps, reduce_motion) = bridge::screen_at(instruction.destination())?;
    let bubble = place_label(instruction.destination(), BUBBLE_SIZE, &screen);
    let shown = if state.style.highlight() {
        instruction.clone()
    } else {
        instruction.clone().with_target(None)
    };
    let frames = frames_for(state, &shown, &screen, fps, reduce_motion);
    bridge::run(&frames, fps, &shown, reduce_motion, &bubble)
}

pub(super) fn frames_for(
    state: &OverlayState,
    instruction: &CursorOverlayInstruction,
    screen: &agent_desktop_core::Rect,
    fps: u32,
    reduce_motion: bool,
) -> Vec<CursorPose> {
    if reduce_motion {
        vec![CursorPose::still(instruction.destination().clone())]
    } else {
        motion_frames(state, instruction, screen, fps)
    }
}

pub(super) fn motion_frames(
    state: &OverlayState,
    instruction: &CursorOverlayInstruction,
    screen: &agent_desktop_core::Rect,
    fps: u32,
) -> Vec<CursorPose> {
    let destination = instruction.destination();
    if instruction.phase() == CursorPhase::Effect {
        let mut frames = vec![CursorPose::still(destination.clone())];
        if instruction.is_click() && state.style.ripple() {
            frames.push(CursorPose {
                point: destination.clone(),
                ripple: 1.0,
            });
        }
        return frames;
    }
    let start = state.at.clone().unwrap_or_else(|| Point {
        x: (destination.x - 180.0).clamp(screen.x, screen.x + screen.width),
        y: (destination.y + 108.0).clamp(screen.y, screen.y + screen.height),
    });
    let motion = CursorMotion::shaped(start, destination.clone(), &state.motion, state.moves)
        .with_impact(instruction.is_click())
        .with_ripple(state.style.ripple());
    let frame_ms = 1_000.0 / f64::from(fps);
    let frame_count = (motion.total_ms() as f64 / frame_ms).ceil() as u64;
    (0..=frame_count)
        .map(|frame| {
            let elapsed = ((frame as f64 * frame_ms).round() as u64).min(motion.total_ms());
            motion.pose(elapsed)
        })
        .collect()
}
