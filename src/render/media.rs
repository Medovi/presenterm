use crate::render::properties::WindowSize;
use crossterm::cursor;
use image::{DynamicImage, ImageError};
use std::{fmt::Debug, fs, io, rc::Rc};
use viuer::ViuError;

/// An image.
///
/// This stores the image in an [std::rc::Rc] so it's cheap to clone.
#[derive(Clone, PartialEq)]
pub struct Image(Rc<DynamicImage>);

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image<{}x{}>", self.0.width(), self.0.height())
    }
}

impl Image {
    /// Construct a new image from a byte sequence.
    pub fn new(contents: &[u8]) -> Result<Self, InvalidImage> {
        let contents = image::load_from_memory(contents)?;
        let contents = Rc::new(contents);
        Ok(Self(contents))
    }
}

/// A media render.
pub struct MediaRender;

impl MediaRender {
    /// Draw an image.
    ///
    /// This will use the current terminal size and try to render the image where the cursor is
    /// currently positioned, respecting the image size. That is, if the image is 300 by 100 pixels
    /// and that fits in the screen at the current cursor positioned, it will be drawn as-is.
    ///
    /// In case the image does not fit, it will be resized to fit the screen, preserving the aspect
    /// ratio.
    pub fn draw_image(&self, image: &Image, dimensions: &WindowSize) -> Result<(), RenderImageError> {
        let position = cursor::position()?;
        let image = &image.0;

        // Compute the image's width in columns by translating pixels -> columns.
        let column_in_pixels = dimensions.pixels_per_column();
        let column_margin = (dimensions.columns as f64 * 0.95) as u32;
        let mut width_in_columns = (image.width() as f64 / column_in_pixels) as u32;

        // Do the same for its height.
        let row_in_pixels = dimensions.pixels_per_row();
        let height_in_rows = (image.height() as f64 / row_in_pixels) as u32;

        // If the image doesn't fit vertically, shrink it.
        let available_height = dimensions.rows.saturating_sub(position.1) as u32;
        if height_in_rows > available_height {
            // Because we only use the width to draw, here we scale the width based on how much we
            // need to shrink the height.
            let shrink_ratio = available_height as f64 / height_in_rows as f64;
            width_in_columns = (width_in_columns as f64 * shrink_ratio) as u32;
        }
        // Don't go too far wide.
        let width_in_columns = width_in_columns.min(column_margin);

        // Draw it in the middle
        let start_column = dimensions.columns / 2 - (width_in_columns / 2) as u16;
        let config = viuer::Config {
            width: Some(width_in_columns),
            x: start_column,
            y: position.1 as i16,
            ..Default::default()
        };
        self.clear_viuer_temp_files();
        viuer::print(image, &config)?;
        Ok(())
    }

    // viuer leaves a bunch of tempfiles when using kitty, this clears them up. Note that because
    // kitty is optional and this is technically not needed for this app to work, we swallow all
    // errors here.
    //
    // See https://github.com/atanunq/viuer/issues/47
    fn clear_viuer_temp_files(&self) {
        let Ok(entries) = fs::read_dir("/tmp") else { return };
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|f| f.to_str()) else { continue };
            if file_name.starts_with(".tmp.viuer.") {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

/// An invalid image.
#[derive(thiserror::Error, Debug)]
#[error("invalid image: {0}")]
pub struct InvalidImage(#[from] ImageError);

/// An image render error.
#[derive(thiserror::Error, Debug)]
#[error("invalid image: {0}")]
pub enum RenderImageError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("draw: {0}")]
    Draw(#[from] ViuError),

    #[error("invalid image: {0}")]
    InvalidImage(#[from] InvalidImage),
}
