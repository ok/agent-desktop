use crate::Role;
use serde::{Deserialize, Serialize};

/// Which cursor image shows: the arrow by default, the pointer over a pressable control and the
/// text caret over a control that takes typed text, as the system cursor does.
#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    #[default]
    Arrow,
    Pointer,
    Text,
}

impl CursorShape {
    /// The shape the cursor shows over an element of `role`.
    pub const fn over(role: Role) -> Self {
        if role.is_pressable() {
            Self::Pointer
        } else if role.takes_text() {
            Self::Text
        } else {
            Self::Arrow
        }
    }

    pub const fn is_arrow(&self) -> bool {
        matches!(self, Self::Arrow)
    }
}
