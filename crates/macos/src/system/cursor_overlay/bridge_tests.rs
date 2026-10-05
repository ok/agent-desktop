use super::*;
use agent_desktop_core::CursorOverlayConfig;

fn instruction(phase: CursorPhase, shape: CursorShape) -> CursorOverlayInstruction {
    let config = CursorOverlayConfig::enabled(None, 6).expect("valid config");
    CursorOverlayInstruction::new(Point { x: 5.0, y: 5.0 }, &config, false)
        .expect("valid instruction")
        .with_phase(phase)
        .with_shape(shape)
}

#[test]
fn travel_departs_as_the_arrow_and_arrives_as_its_shape() {
    for (shape, slot) in [
        (CursorShape::Arrow, ARROW_SLOT),
        (CursorShape::Pointer, POINTER_SLOT),
        (CursorShape::Text, TEXT_SLOT),
    ] {
        let flags = pointer_flags(&instruction(CursorPhase::Travel, shape));
        assert_ne!(flags & ARROW_ON_DEPARTURE, 0, "{shape:?}");
        assert_eq!((flags & SHAPE_ON_ARRIVAL) >> SHAPE_SHIFT, slot, "{shape:?}");
    }
}

#[test]
fn drags_use_the_arrow_and_effects_keep_the_current_image() {
    assert_eq!(
        pointer_flags(&instruction(CursorPhase::Drag, CursorShape::Pointer)),
        ARROW_ON_DEPARTURE
    );
    for shape in [CursorShape::Arrow, CursorShape::Pointer, CursorShape::Text] {
        assert_eq!(pointer_flags(&instruction(CursorPhase::Effect, shape)), 0);
    }
}

#[test]
fn pointer_flags_do_not_collide_with_render_flags() {
    let all = [
        REDUCE_MOTION,
        HIGHLIGHT,
        SHAPE_ON_ARRIVAL,
        ARROW_ON_DEPARTURE,
    ];
    for (index, flag) in all.iter().enumerate() {
        for other in &all[index + 1..] {
            assert_eq!(flag & other, 0, "{flag:#b} overlaps {other:#b}");
        }
    }
}
