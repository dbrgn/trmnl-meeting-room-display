use image::{ImageBuffer, Luma, codecs::bmp::BmpEncoder};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

/// Generate a monochrome 800x480 BMP with "hello world" text
pub fn generate_hello_world_bmp() -> Result<Vec<u8>, Box<dyn Error>> {
    // Create an 800x480 monochrome image (8-bit for simplicity)
    let width = 800;
    let height = 480;
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(width, height);

    // Fill with white background
    for pixel in img.pixels_mut() {
        *pixel = Luma([255]); // White
    }

    // Load font
    let font_path = Path::new("assets/fonts/BlockKie.ttf");
    let mut font_data = Vec::new();
    File::open(font_path)?.read_to_end(&mut font_data)?;

    let font = Font::try_from_bytes(&font_data).ok_or_else(|| "Error loading font".to_string())?;

    let text = "hello world";

    // Configure text scale (font size)
    let scale = Scale { x: 50.0, y: 50.0 };

    // Calculate text dimensions to center it
    let v_metrics = font.v_metrics(scale);
    let text_width = font
        .layout(text, scale, rusttype::point(0.0, 0.0))
        .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
        .last()
        .unwrap_or(0.0);

    // Position text in the center of the image
    let x = ((width as f32 - text_width) / 2.0).floor() as i32;
    let y = ((height as f32 - v_metrics.ascent + v_metrics.descent) / 2.0).floor() as i32;

    // Draw text
    draw_text_mut(
        &mut img,
        Luma([0]), // Black text
        x,
        y,
        scale,
        &font,
        text,
    );

    // Draw a border around the text for visibility
    let border_padding = 20;
    let text_height = (v_metrics.ascent - v_metrics.descent) as i32;

    let border_x = x - border_padding;
    let border_y = y - border_padding;
    let border_width = text_width as i32 + (2 * border_padding);
    let border_height = text_height + (2 * border_padding);

    // Draw border
    for ix in border_x..(border_x + border_width) {
        if ix >= 0 && ix < width as i32 {
            // Top border
            if border_y >= 0 && border_y < height as i32 {
                img.put_pixel(ix as u32, border_y as u32, Luma([0]));
            }
            // Bottom border
            if (border_y + border_height) >= 0 && (border_y + border_height) < height as i32 {
                img.put_pixel(ix as u32, (border_y + border_height) as u32, Luma([0]));
            }
        }
    }

    for iy in border_y..(border_y + border_height) {
        if iy >= 0 && iy < height as i32 {
            // Left border
            if border_x >= 0 && border_x < width as i32 {
                img.put_pixel(border_x as u32, iy as u32, Luma([0]));
            }
            // Right border
            if (border_x + border_width) >= 0 && (border_x + border_width) < width as i32 {
                img.put_pixel((border_x + border_width) as u32, iy as u32, Luma([0]));
            }
        }
    }

    // Convert to monochrome BMP
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = BmpEncoder::new(&mut cursor);

    // Encode the image
    encoder
        .encode(&img.to_vec(), width, height, image::ColorType::L8)
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;

    Ok(cursor.into_inner())
}
