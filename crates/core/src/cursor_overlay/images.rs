use super::CursorImage;
use crate::AdapterError;
use serde::{Deserialize, Serialize};

/// Custom cursor artwork: `arrow` is drawn while the cursor travels and rests, `pointer` over a
/// pressable control and `text` over a control that takes typed text. Any may be absent; a
/// missing arrow falls back to the built-in cursor, a missing pointer or text image keeps the
/// arrow.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CursorImages {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    arrow: Option<CursorImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pointer: Option<CursorImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<CursorImage>,
}

impl CursorImages {
    pub const fn is_empty(&self) -> bool {
        self.arrow.is_none() && self.pointer.is_none() && self.text.is_none()
    }

    pub fn validated(self) -> Result<Self, AdapterError> {
        Ok(Self {
            arrow: self.arrow.map(CursorImage::validated).transpose()?,
            pointer: self.pointer.map(CursorImage::validated).transpose()?,
            text: self.text.map(CursorImage::validated).transpose()?,
        })
    }

    pub fn set_arrow(&mut self, arrow: Option<CursorImage>) {
        self.arrow = arrow;
    }

    pub fn set_pointer(&mut self, pointer: Option<CursorImage>) {
        self.pointer = pointer;
    }

    pub fn set_text(&mut self, text: Option<CursorImage>) {
        self.text = text;
    }

    pub const fn arrow(&self) -> Option<&CursorImage> {
        self.arrow.as_ref()
    }

    pub const fn pointer(&self) -> Option<&CursorImage> {
        self.pointer.as_ref()
    }

    pub const fn text(&self) -> Option<&CursorImage> {
        self.text.as_ref()
    }
}
