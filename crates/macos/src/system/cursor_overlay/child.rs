use agent_desktop_core::{
    AdapterError, CURSOR_IDLE_REST_MS, CursorMotionProfile, CursorOverlayConfig,
    CursorOverlayControl, CursorOverlayInstruction, CursorOverlayStyle, CursorPhase, ErrorCode,
    Point,
};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use super::bridge;
use super::settle::{self, ShapeSettle};
use render::render;

#[path = "child_aim.rs"]
mod aim;
#[path = "child_render.rs"]
mod render;

pub(super) const MARKER: &str = "AGENT_DESKTOP_CURSOR_OVERLAY_CHILD";
pub(super) const SOCKET_ENV: &str = "AGENT_DESKTOP_CURSOR_OVERLAY_SOCKET";
const MAX_INSTRUCTION_BYTES: u64 = 4 * 1024;
const BUBBLE_SIZE: (f64, f64) = (232.0, 38.0);

#[derive(Default)]
struct OverlayState {
    style: CursorOverlayStyle,
    motion: CursorMotionProfile,
    moves: u64,
    at: Option<Point>,
    resting: bool,
    settle: ShapeSettle,
    drift: Option<aim::Drift>,
}

pub(crate) fn entry_from_env() -> Option<Result<(), AdapterError>> {
    match std::env::var(MARKER) {
        Err(_) => None,
        Ok(value) if value == super::endpoint::PROTOCOL_VERSION => Some(run()),
        Ok(_) => Some(Err(AdapterError::internal(
            "Invalid cursor overlay child protocol marker",
        ))),
    }
}

fn run() -> Result<(), AdapterError> {
    let initial = read_control(std::io::stdin())?;
    if !session_active(initial.session_id(), initial.agent_id()) {
        return Ok(());
    }
    let socket = socket_path(&initial)?;
    prepare_socket(&socket)?;
    let mut state = OverlayState::default();
    if !handle(&initial, &mut state)? {
        return cleanup(socket);
    }
    let listener = UnixListener::bind(&socket).map_err(|error| {
        AdapterError::internal("Could not bind the cursor overlay session socket")
            .with_platform_detail(error.to_string())
    })?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).map_err(|error| {
        AdapterError::internal("Could not protect the cursor overlay session socket")
            .with_platform_detail(error.to_string())
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        AdapterError::internal("Could not configure the cursor overlay session socket")
            .with_platform_detail(error.to_string())
    })?;
    let mut quiet_since = std::time::Instant::now();
    let mut checked_session = quiet_since;
    loop {
        if checked_session.elapsed() >= Duration::from_secs(1) {
            if !session_active(initial.session_id(), initial.agent_id()) {
                return cleanup(socket);
            }
            checked_session = std::time::Instant::now();
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                if prepare_stream(&stream).is_err() {
                    continue;
                }
                let Ok(control) = read_control(&mut stream) else {
                    continue;
                };
                if control.session_id() != initial.session_id()
                    || (control.agent_id() != initial.agent_id()
                        && !(control.agent_id().is_none()
                            && (control.is_transient() || control.is_disable())))
                {
                    continue;
                }
                if control.is_disable() {
                    drop(listener);
                    cleanup(socket)?;
                    let _ = stream.write_all(&[1]);
                    return Ok(());
                }
                quiet_since = std::time::Instant::now();
                if state.resting {
                    bridge::show();
                    state.resting = false;
                }
                let outcome = handle(&control, &mut state);
                if outcome.is_ok() && control.expects_acknowledgement() {
                    let _ = stream.write_all(&[1]);
                }
                if outcome.is_err() {
                    continue;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                bridge::idle();
                let _ = aim::drift_when_due(&mut state);
                settle_shape(&mut state);
                if !state.resting
                    && !bridge::drag_active()
                    && quiet_since.elapsed().as_millis() >= u128::from(CURSOR_IDLE_REST_MS)
                {
                    bridge::rest();
                    state.resting = true;
                    state.settle.cancel();
                }
                thread::sleep(Duration::from_millis(8));
            }
            Err(error) => {
                let _ = cleanup(socket);
                return Err(
                    AdapterError::internal("Cursor overlay session socket failed")
                        .with_platform_detail(error.to_string()),
                );
            }
        }
    }
}

fn prepare_stream(stream: &UnixStream) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))
}

fn handle(control: &CursorOverlayControl, state: &mut OverlayState) -> Result<bool, AdapterError> {
    control.validate()?;
    if control.is_disable() {
        return Ok(false);
    }
    if let Some(image_changed) = absorb_settings(control, state) {
        bridge::apply_style(&state.style, image_changed);
    }
    state.drift = None;
    if control.is_hide() {
        bridge::hide();
        state.settle.cancel();
        apply_landing_memory(control, state, None);
        return Ok(true);
    }
    if control.is_show() {
        bridge::show();
        apply_landing_memory(control, state, None);
        return Ok(true);
    }
    let owned;
    let instruction = if let Some(instruction) = control.instruction() {
        instruction
    } else {
        let config = CursorOverlayConfig::enabled(control.label().map(str::to_owned), 12)?;
        owned = CursorOverlayInstruction::new(bridge::initial_point()?, &config, false)?;
        &owned
    };
    let aimed = aim::aimed(instruction, state);
    render(&aimed, state)?;
    apply_landing_memory(control, state, Some(&aimed));
    advance_moves(control, state);
    aim::plan_drift(&aimed, state);
    if instruction.phase() == CursorPhase::Drag || !settle::has_shapes(&state.style) {
        state.settle.cancel();
    } else {
        state.settle.schedule(std::time::Instant::now());
    }
    Ok(true)
}

fn settle_shape(state: &mut OverlayState) {
    if state.settle.due(std::time::Instant::now())
        && !bridge::drag_active()
        && let Some(at) = &state.at
    {
        settle::apply(at);
    }
}

fn absorb_settings(control: &CursorOverlayControl, state: &mut OverlayState) -> Option<bool> {
    if let Some(motion) = control.motion() {
        state.motion = motion.clone();
    }
    if control.is_enable() {
        state.moves = 0;
    }
    absorb_style(control, state)
}

fn advance_moves(control: &CursorOverlayControl, state: &mut OverlayState) {
    if control
        .instruction()
        .is_some_and(|instruction| instruction.phase() == CursorPhase::Travel)
    {
        state.moves = state.moves.wrapping_add(1);
    }
}

fn absorb_style(control: &CursorOverlayControl, state: &mut OverlayState) -> Option<bool> {
    let style = control.style()?;
    let image_changed = state.style.images() != style.images();
    state.style = style.clone();
    Some(image_changed)
}

fn apply_landing_memory(
    control: &CursorOverlayControl,
    state: &mut OverlayState,
    instruction: Option<&CursorOverlayInstruction>,
) {
    if control.is_hide() {
        state.at = None;
        return;
    }
    if control.is_show() {
        return;
    }
    let Some(instruction) = instruction else {
        return;
    };
    state.at = Some(if instruction.phase() == CursorPhase::Drag {
        instruction
            .drag_from()
            .unwrap_or(instruction.destination())
            .clone()
    } else {
        instruction.destination().clone()
    });
}

fn read_control(reader: impl Read) -> Result<CursorOverlayControl, AdapterError> {
    let mut payload = Vec::new();
    reader
        .take(MAX_INSTRUCTION_BYTES + 1)
        .read_to_end(&mut payload)
        .map_err(|error| {
            AdapterError::internal("Could not read cursor overlay control")
                .with_platform_detail(error.to_string())
        })?;
    if payload.len() as u64 > MAX_INSTRUCTION_BYTES {
        return Err(AdapterError::internal(
            "Cursor overlay instruction exceeds the transport limit",
        ));
    }
    let control: CursorOverlayControl = serde_json::from_slice(&payload).map_err(|error| {
        AdapterError::internal("Could not decode cursor overlay control")
            .with_platform_detail(error.to_string())
    })?;
    control.validate()?;
    Ok(control)
}

fn socket_path(control: &CursorOverlayControl) -> Result<PathBuf, AdapterError> {
    let expected = super::endpoint::path(control.session_id(), control.agent_id())?;
    let supplied = std::env::var_os(SOCKET_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| AdapterError::internal("Cursor overlay child socket is missing"))?;
    if supplied != expected {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            "Cursor overlay child socket does not match its session",
        ));
    }
    Ok(expected)
}

pub(super) fn session_active(session_id: &str, agent_id: Option<&str>) -> bool {
    agent_desktop_core::session::read_manifest(session_id)
        .ok()
        .flatten()
        .is_some_and(|manifest| {
            manifest.ended_at.is_none()
                && manifest.cursor_overlay.is_enabled()
                && !(manifest.cursor_overlay.is_multi_agent() && agent_id.is_none())
        })
}

fn prepare_socket(path: &Path) -> Result<(), AdapterError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(
            AdapterError::internal("Could not replace stale cursor overlay socket")
                .with_platform_detail(error.to_string()),
        ),
    }
}

fn cleanup(path: PathBuf) -> Result<(), AdapterError> {
    bridge::stop();
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(
            AdapterError::internal("Could not remove cursor overlay socket")
                .with_platform_detail(error.to_string()),
        ),
    }
}

#[cfg(test)]
#[path = "child_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "child_motion_tests.rs"]
mod motion_tests;

#[cfg(test)]
#[path = "child_image_tests.rs"]
mod image_tests;
