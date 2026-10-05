use agent_desktop_core::{CursorShape, Point};

const ANCESTOR_LIMIT: usize = 8;

#[cfg(target_os = "macos")]
mod imp {
    use super::{ANCESTOR_LIMIT, CursorShape, Point};
    use crate::tree::AXElement;
    use accessibility_sys::{AXUIElementCreateSystemWide, kAXErrorSuccess, kAXRoleAttribute};
    use std::time::Instant;

    /// The shape the cursor shows over whatever lies at `point`, read from the element there
    /// and its nearest ancestors: a label inside a button shows the button's pointer. `None`
    /// when nothing answers in time, or the hit is this process's own overlay.
    pub(crate) fn shape_at(point: &Point, deadline: Instant) -> Option<CursorShape> {
        let system = AXElement(unsafe { AXUIElementCreateSystemWide() });
        if system.0.is_null() {
            return None;
        }
        let (error, hit) = crate::tree::ax_ipc::element_at_position(
            &system,
            (point.x as f32, point.y as f32),
            deadline,
        );
        if error != kAXErrorSuccess || hit.is_null() {
            return None;
        }
        let mut current = AXElement(hit);
        let pid = crate::tree::ax_ipc::pid(&current, deadline).ok()?;
        if u32::try_from(pid).ok() == Some(std::process::id()) {
            return None;
        }
        for _ in 0..ANCESTOR_LIMIT {
            let role =
                crate::tree::resolve_ax_read::read_string(&current, kAXRoleAttribute, deadline)
                    .ok()??;
            if ends_walk(&role) {
                return Some(CursorShape::Arrow);
            }
            let subrole =
                crate::tree::resolve_ax_read::read_string(&current, "AXSubrole", deadline)
                    .ok()
                    .flatten();
            let shape = super::shape_for(&role, subrole.as_deref());
            if !shape.is_arrow() {
                return Some(shape);
            }
            current =
                match crate::tree::resolve_ax_read::read_element(&current, "AXParent", deadline) {
                    Ok(Some(parent)) => parent,
                    Ok(None) => return Some(CursorShape::Arrow),
                    Err(_) => return None,
                };
        }
        Some(CursorShape::Arrow)
    }

    fn ends_walk(role: &str) -> bool {
        matches!(
            role,
            "AXWebArea" | "AXWindow" | "AXSheet" | "AXApplication" | "AXScrollArea"
        )
    }
}

/// A pop-up button reports the combo box role but is pressed, not typed into.
fn shape_for(ax_role: &str, ax_subrole: Option<&str>) -> CursorShape {
    if ax_role == "AXPopUpButton" {
        return CursorShape::Pointer;
    }
    CursorShape::over(agent_desktop_core::Role::from_token(
        crate::tree::roles::ax_role_and_subrole_to_str(ax_role, ax_subrole),
    ))
}

#[cfg(target_os = "macos")]
pub(crate) use imp::shape_at;

#[cfg(test)]
mod tests {
    use super::shape_for;
    use agent_desktop_core::CursorShape;

    #[test]
    fn controls_map_to_the_shape_the_system_cursor_shows() {
        for (role, subrole, shape) in [
            ("AXButton", None, CursorShape::Pointer),
            ("AXLink", None, CursorShape::Pointer),
            ("AXCheckBox", Some("AXToggleButton"), CursorShape::Pointer),
            ("AXRadioButton", Some("AXTabButton"), CursorShape::Pointer),
            ("AXPopUpButton", None, CursorShape::Pointer),
            ("AXTextField", None, CursorShape::Text),
            ("AXTextArea", None, CursorShape::Text),
            ("AXTextField", Some("AXSearchField"), CursorShape::Text),
            ("AXComboBox", None, CursorShape::Text),
            ("AXStaticText", None, CursorShape::Arrow),
            ("AXGroup", None, CursorShape::Arrow),
            ("AXImage", None, CursorShape::Arrow),
        ] {
            assert_eq!(shape_for(role, subrole), shape, "{role} {subrole:?}");
        }
    }
}
