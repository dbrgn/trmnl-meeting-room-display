use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result};
use image::{ImageBuffer, Luma, codecs::bmp::BmpEncoder};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

/// Configuration for image generation
pub struct ImageConfig {
    /// Width of the image
    pub width: u32,
    /// Height of the image
    pub height: u32,
    /// Path to the font file
    pub font_path: String,
    /// Font size
    pub font_size: f32,
    /// Text to display
    pub text: String,
    /// Border padding around the text
    pub border_padding: i32,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 480,
            font_path: "assets/fonts/BlockKie.ttf".to_string(),
            font_size: 50.0,
            text: "hello world".to_string(),
            border_padding: 20,
        }
    }
}

/// Generate a monochrome BMP with text using the given configuration
pub fn generate_bmp(config: &ImageConfig) -> Result<Vec<u8>> {
    // Create the image buffer
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(config.width, config.height);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Luma([255]); // White
    }

    // Load font
    let font_path = Path::new(&config.font_path);
    let mut font_data = Vec::new();
    File::open(font_path)
        .with_context(|| format!("Failed to open font file at {}", &config.font_path))?
        .read_to_end(&mut font_data)
        .context("Failed to read font data")?;

    let font = Font::try_from_bytes(&font_data)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse font data"))?;

    // Configure text scale (font size)
    let scale = Scale {
        x: config.font_size,
        y: config.font_size,
    };

    // Calculate text dimensions to center it
    let v_metrics = font.v_metrics(scale);
    let text_width = font
        .layout(&config.text, scale, rusttype::point(0.0, 0.0))
        .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
        .last()
        .unwrap_or(0.0);

    // Position text in the center of the image
    let x = ((config.width as f32 - text_width) / 2.0).floor() as i32;
    let y = ((config.height as f32 - v_metrics.ascent + v_metrics.descent) / 2.0).floor() as i32;

    // Draw text
    draw_text_mut(
        &mut img,
        Luma([0]), // Black text
        x,
        y,
        scale,
        &font,
        &config.text,
    );

    // Draw a border around the text for visibility
    let text_height = (v_metrics.ascent - v_metrics.descent) as i32;

    let border_x = x - config.border_padding;
    let border_y = y - config.border_padding;
    let border_width = text_width as i32 + (2 * config.border_padding);
    let border_height = text_height + (2 * config.border_padding);

    draw_border(
        &mut img,
        border_x,
        border_y,
        border_width,
        border_height,
        config.width,
        config.height,
    );

    // Convert to monochrome BMP
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = BmpEncoder::new(&mut cursor);

    // Encode the image
    encoder
        .encode(&img, config.width, config.height, image::ColorType::L8)
        .context("Failed to encode BMP image")?;

    Ok(cursor.into_inner())
}

/// Draw a border around the specified rectangle
fn draw_border(
    img: &mut ImageBuffer<Luma<u8>, Vec<u8>>,
    border_x: i32,
    border_y: i32,
    border_width: i32,
    border_height: i32,
    img_width: u32,
    img_height: u32,
) {
    // Draw border
    for ix in border_x..(border_x + border_width) {
        if ix >= 0 && ix < img_width as i32 {
            // Top border
            if border_y >= 0 && border_y < img_height as i32 {
                img.put_pixel(ix as u32, border_y as u32, Luma([0]));
            }
            // Bottom border
            if (border_y + border_height) >= 0 && (border_y + border_height) < img_height as i32 {
                img.put_pixel(ix as u32, (border_y + border_height) as u32, Luma([0]));
            }
        }
    }

    for iy in border_y..(border_y + border_height) {
        if iy >= 0 && iy < img_height as i32 {
            // Left border
            if border_x >= 0 && border_x < img_width as i32 {
                img.put_pixel(border_x as u32, iy as u32, Luma([0]));
            }
            // Right border
            if (border_x + border_width) >= 0 && (border_x + border_width) < img_width as i32 {
                img.put_pixel((border_x + border_width) as u32, iy as u32, Luma([0]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bmp_with_default_config() {
        let result = generate_bmp(&ImageConfig::default());
        assert!(
            result.is_ok(),
            "Failed to generate BMP with default config: {:?}",
            result.err()
        );
        let bmp_data = result.unwrap();
        assert!(!bmp_data.is_empty(), "Generated BMP data is empty");
    }

    #[test]
    fn test_generate_bmp_with_custom_config() {
        let config = ImageConfig {
            width: 400,
            height: 240,
            font_path: "assets/fonts/BlockKie.ttf".to_string(),
            font_size: 25.0,
            text: "test image".to_string(),
            border_padding: 10,
        };

        let result = generate_bmp(&config);
        assert!(
            result.is_ok(),
            "Failed to generate BMP with custom config: {:?}",
            result.err()
        );
        let bmp_data = result.unwrap();
        assert!(!bmp_data.is_empty(), "Generated BMP data is empty");
    }
}
