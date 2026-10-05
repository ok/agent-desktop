use crate::{AdapterError, ErrorCode};
use serde::{Deserialize, Serialize};

/// Where on a control the drawn cursor lands and where it goes once a click has played.
/// Presentation only: a semantic action does not depend on the drawn point, and physical
/// delivery keeps aiming at its verified point.
///
/// `spread` lets the landing point stray from the control's centre by up to that fraction of
/// its half-width and half-height, weighted towards the centre. `drift` moves the cursor off a
/// clicked control to a nearby spot when no further action follows, as a person clears the
/// view after clicking.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CursorAim {
    #[serde(default)]
    spread: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    drift: bool,
}

impl CursorAim {
    pub fn validate(&self) -> Result<(), AdapterError> {
        if self.spread.is_finite() && (0.0..=1.0).contains(&self.spread) {
            Ok(())
        } else {
            Err(AdapterError::new(
                ErrorCode::InvalidArgs,
                "Cursor --aim-spread must be between 0 and 1",
            ))
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn set_spread(&mut self, spread: f64) {
        self.spread = spread;
    }

    pub fn set_drift(&mut self, drift: bool) {
        self.drift = drift;
    }

    pub const fn spread(&self) -> f64 {
        self.spread
    }

    pub const fn drift(&self) -> bool {
        self.drift
    }
}
