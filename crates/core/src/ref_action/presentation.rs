use super::{ActionabilityPreflight, ResolvedRefAction};
use crate::cursor_overlay::CursorPhase;
use crate::{Action, CommandContext, PlatformAdapter};

pub(super) fn before_dispatch(
    target: &ResolvedRefAction<'_>,
    preflight: &ActionabilityPreflight,
    action: &Action,
    lease: &crate::InteractionLease,
) {
    let Some(destination) = destination(preflight) else {
        return;
    };
    let shape = arrival_shape(&target.entry.identity.role, action);
    crate::cursor_overlay::submit_travel(target.adapter, target.context, destination, lease, shape);
}

/// A pressable control shows the pointer and a text control the text caret however either is
/// reached; any other element shows the pointer only when clicked.
pub(super) fn arrival_shape(role: &str, action: &Action) -> crate::CursorShape {
    match crate::CursorShape::over(crate::Role::from_token(role)) {
        crate::CursorShape::Arrow if is_click(action) => crate::CursorShape::Pointer,
        shape => shape,
    }
}

pub(super) fn after_dispatch(
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
    preflight: &ActionabilityPreflight,
    action: &Action,
    result: &Result<crate::ActionResult, crate::AdapterError>,
) {
    let disposition = match result {
        Ok(result) => result.disposition(),
        Err(error) => error.disposition,
    };
    if !crate::cursor_overlay::confirms_delivery(disposition) {
        return;
    }
    let Some(destination) = destination(preflight) else {
        return;
    };
    crate::cursor_overlay::submit(
        adapter,
        context,
        destination,
        preflight.presentation_bounds,
        is_click(action),
        CursorPhase::Effect,
    );
}

fn destination(preflight: &ActionabilityPreflight) -> Option<crate::Point> {
    match preflight.pointer_delivery {
        crate::actionability::PointerDelivery::Physical => preflight
            .verified_point
            .clone()
            .or_else(|| preflight.presentation_point.clone()),
        _ => preflight.presentation_point.clone(),
    }
}

fn is_click(action: &Action) -> bool {
    matches!(
        action,
        Action::Click | Action::DoubleClick | Action::RightClick | Action::TripleClick
    )
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
