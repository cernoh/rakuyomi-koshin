//! QR code image generation for the auth-url UX.
//!
//! The e-ink reader renders a 300×300 greyscale PNG of the OAuth
//! authorization URL. The user scans the code with their phone, which
//! completes the OAuth flow in a real browser and returns a token.

use std::io::Cursor;

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Luma};

use crate::AppError;

/// Encode `url` as a 300×300 greyscale PNG QR code.
///
/// Returns the raw PNG byte buffer. Caller is responsible for setting
/// the `Content-Type: image/png` response header and choosing the
/// cache policy (we don't set caching headers here because QR images
/// are tied to a 15-minute PKCE session).
pub fn encode_url_to_qr_png(url: &str) -> Result<Vec<u8>, AppError> {
    let code = qrcode::QrCode::new(url.as_bytes()).map_err(|e| AppError::Other(e.into()))?;

    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(300, 300)
        .build();

    let mut png_bytes = Vec::new();
    PngEncoder::new(Cursor::new(&mut png_bytes))
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::L8,
        )
        .map_err(|e| AppError::Other(e.into()))?;

    Ok(png_bytes)
}
