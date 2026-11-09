use std::io::Cursor;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::{DynamicImage, ImageFormat, codecs::jpeg::JpegEncoder};
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};

const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIMENSION: u32 = 2048;

pub struct PreparedImage {
    pub mime_type: String,
    pub data_base64: Zeroizing<String>,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub original_hash: String,
    pub sent_hash: String,
}

pub fn prepare_image(bytes: &[u8]) -> AppResult<PreparedImage> {
    if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES {
        return Err(AppError::Validation(
            "Images must be between 1 byte and 8 MB.".into(),
        ));
    }
    let format = image::guess_format(bytes)
        .map_err(|_| AppError::Validation("Unsupported or invalid image file.".into()))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return Err(AppError::Validation(
            "Only PNG, JPEG, and WebP images are supported.".into(),
        ));
    }
    let image = image::load_from_memory_with_format(bytes, format)
        .map_err(|_| AppError::Validation("Unable to decode the image.".into()))?;
    let image = resize(image);
    let (width, height) = (image.width(), image.height());
    let (mime_type, encoded) = encode(image, format)?;
    if encoded.len() > MAX_OUTPUT_BYTES {
        return Err(AppError::Validation(
            "The image is too large to send. Crop or reduce it below 4 MB.".into(),
        ));
    }
    let original_hash = blake3::hash(bytes).to_hex().to_string();
    let sent_hash = blake3::hash(&encoded).to_hex().to_string();
    Ok(PreparedImage {
        mime_type: mime_type.into(),
        data_base64: Zeroizing::new(BASE64.encode(&encoded)),
        size: encoded.len(),
        width,
        height,
        original_hash,
        sent_hash,
    })
}

fn resize(image: DynamicImage) -> DynamicImage {
    if image.width() <= MAX_DIMENSION && image.height() <= MAX_DIMENSION {
        image
    } else {
        image.resize(
            MAX_DIMENSION,
            MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
    }
}

fn encode(image: DynamicImage, source_format: ImageFormat) -> AppResult<(&'static str, Vec<u8>)> {
    if source_format == ImageFormat::Jpeg {
        let mut out = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut out, 85);
        encoder
            .encode_image(&image)
            .map_err(|_| AppError::Internal("Unable to encode the image.".into()))?;
        return Ok(("image/jpeg", out));
    }
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|_| AppError::Internal("Unable to encode the image.".into()))?;
    Ok(("image/png", cursor.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    fn reencodes_small_png_and_strips_metadata() {
        let mut image = RgbaImage::new(4, 3);
        image.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .expect("png");
        let prepared = prepare_image(bytes.get_ref()).expect("prepared");
        assert_eq!(prepared.mime_type, "image/png");
        assert_eq!((prepared.width, prepared.height), (4, 3));
        assert!(!prepared.data_base64.is_empty());
        assert_eq!(prepared.original_hash, prepared.sent_hash);
    }

    #[test]
    fn rejects_non_image_data() {
        assert!(prepare_image(b"not an image").is_err());
    }
}
