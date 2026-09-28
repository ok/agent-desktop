use agent_desktop_core::{AppError, CursorImage, CursorOverlayStyle, Point};
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug, Default)]
pub(crate) struct CursorOverlayStyleArgs {
    #[arg(
        long,
        conflicts_with = "image",
        help = "Cursor body colour as a hex value (default #FFFFFF)"
    )]
    pub fill: Option<String>,
    #[arg(
        long,
        conflicts_with = "image",
        help = "Cursor outline colour as a hex value (default #111318)"
    )]
    pub rim: Option<String>,
    #[arg(
        long,
        help = "Ripple and element highlight colour as a hex value (default #4299FF)"
    )]
    pub accent: Option<String>,
    #[arg(long, help = "Cursor size multiplier from 0.5 to 4.0 (default 1.0)")]
    pub size: Option<f64>,
    #[arg(long, help = "Do not play the ripple when the agent clicks")]
    pub no_ripple: bool,
    #[arg(long, help = "Do not outline the element when the agent clicks")]
    pub no_highlight: bool,
    #[arg(
        long,
        value_name = "PATH",
        help = "Draw the cursor from a PNG or PDF file instead of the built-in arrow; scaled by --size"
    )]
    pub image: Option<PathBuf>,
    #[arg(
        long,
        value_name = "X,Y",
        requires = "image",
        value_parser = parse_hotspot,
        help = "Point in the image, in image points from its top-left, that marks the click position (default 0,0)"
    )]
    pub hotspot: Option<(f64, f64)>,
    #[arg(
        long,
        value_name = "PATH",
        help = "PNG or PDF shown instead while the cursor rests on a button, link or other pressable control"
    )]
    pub pointer_image: Option<PathBuf>,
    #[arg(
        long,
        value_name = "X,Y",
        requires = "pointer_image",
        value_parser = parse_hotspot,
        help = "Click point in the pointer image, in image points from its top-left (default 0,0)"
    )]
    pub pointer_hotspot: Option<(f64, f64)>,
}

impl CursorOverlayStyleArgs {
    pub(crate) fn to_core(&self) -> Result<CursorOverlayStyle, AppError> {
        let mut style = CursorOverlayStyle::default();
        if let Some(fill) = self.fill.clone() {
            style.set_fill(fill);
        }
        if let Some(rim) = self.rim.clone() {
            style.set_rim(rim);
        }
        if let Some(accent) = self.accent.clone() {
            style.set_accent(accent);
        }
        if let Some(size) = self.size {
            style.set_size(size);
        }
        style.set_effects(!self.no_ripple, !self.no_highlight);
        style.set_image(cursor_image(self.image.as_deref(), self.hotspot)?);
        style.set_pointer_image(cursor_image(
            self.pointer_image.as_deref(),
            self.pointer_hotspot,
        )?);
        Ok(style)
    }
}

fn cursor_image(
    path: Option<&std::path::Path>,
    hotspot: Option<(f64, f64)>,
) -> Result<Option<CursorImage>, AppError> {
    let Some(path) = path else {
        return Ok(None);
    };
    let resolved = std::fs::canonicalize(path).map_err(|error| {
        AppError::invalid_input(format!(
            "Cursor image '{}' cannot be read: {error}",
            path.display()
        ))
    })?;
    let resolved = resolved
        .into_os_string()
        .into_string()
        .map_err(|_| AppError::invalid_input("Cursor image path must be valid UTF-8"))?;
    let (x, y) = hotspot.unwrap_or((0.0, 0.0));
    let image = CursorImage::new(resolved, Point { x, y })?;
    image.verify_file()?;
    Ok(Some(image))
}

fn parse_hotspot(value: &str) -> Result<(f64, f64), String> {
    let invalid = || "hotspot must be X,Y in image points, e.g. 4,2".to_string();
    let (x, y) = value.split_once(',').ok_or_else(invalid)?;
    let x = x.trim().parse::<f64>().map_err(|_| invalid())?;
    let y = y.trim().parse::<f64>().map_err(|_| invalid())?;
    Ok((x, y))
}

#[cfg(test)]
#[path = "cursor_overlay_image_tests.rs"]
mod tests;
