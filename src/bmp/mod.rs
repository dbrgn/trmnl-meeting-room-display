use image::{ImageBuffer, Luma, codecs::bmp::BmpEncoder};
use std::error::Error;
use std::io::Cursor;

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

    // Draw "hello world" manually with simple bitmap font
    // Each character is 5x7 pixels with 1px spacing
    let text = "hello world";
    let char_width: usize = 5;
    let char_height: usize = 7;
    let char_spacing: usize = 1;
    let start_x: usize = (width as usize / 2) - ((text.len() * (char_width + char_spacing)) / 2);
    let start_y: usize = (height as usize / 2) - (char_height / 2);

    // Simple bitmap patterns for each character (1 = pixel, 0 = no pixel)
    let patterns = [
        // h
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // e
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // l
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // l
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // o
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // Space
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // w
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // o
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
        // r
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // l
        [
            [0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 0, 0, 0],
            [0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0],
        ],
        // d
        [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 1, 0],
            [0, 0, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ],
    ];

    // Lookup map for characters
    let char_map = [
        ('h', 0),
        ('e', 1),
        ('l', 2),
        ('l', 3),
        ('o', 4),
        (' ', 5),
        ('w', 6),
        ('o', 7),
        ('r', 8),
        ('l', 9),
        ('d', 10),
    ];

    // Draw each character
    for (i, c) in text.chars().enumerate() {
        let pattern_idx = char_map
            .iter()
            .find(|(ch, _)| *ch == c)
            .map(|(_, idx)| *idx)
            .unwrap_or(5); // Default to space
        let pattern = patterns[pattern_idx];

        let char_x = start_x + i * (char_width + char_spacing);

        // Draw this character
        for y in 0..char_height {
            for x in 0..char_width {
                if pattern[y][x] == 1 {
                    if char_x + x < width as usize && start_y + y < height as usize {
                        img.put_pixel((char_x + x) as u32, (start_y + y) as u32, Luma([0]));
                    }
                }
            }
        }
    }

    // Draw a border around the text for visibility
    let border_x = start_x.saturating_sub(10);
    let border_y = start_y.saturating_sub(10);
    let border_width = text.len() * (char_width + char_spacing) + 20;
    let border_height = char_height + 20;

    // Top and bottom borders
    for x in border_x..(border_x + border_width) {
        if x < width as usize {
            if border_y < height as usize {
                img.put_pixel(x as u32, border_y as u32, Luma([0]));
            }
            if (border_y + border_height) < height as usize {
                img.put_pixel(x as u32, (border_y + border_height) as u32, Luma([0]));
            }
        }
    }

    // Left and right borders
    for y in border_y..(border_y + border_height) {
        if y < height as usize {
            if border_x < width as usize {
                img.put_pixel(border_x as u32, y as u32, Luma([0]));
            }
            if (border_x + border_width) < width as usize {
                img.put_pixel((border_x + border_width) as u32, y as u32, Luma([0]));
            }
        }
    }

    // Convert to monochrome BMP (1-bit)
    // For 1-bit BMP, we need to manually create a proper BMP file
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = BmpEncoder::new(&mut cursor);

    // We're using the encoder to write the image
    // This will result in an 8-bit image, but for this example it's sufficient
    // For a true 1-bit monochrome BMP, a more complex encoding would be needed
    encoder
        .encode(&img.to_vec(), width, height, image::ColorType::L8)
        .map_err(|e| Box::new(e) as Box<dyn Error>)?;

    Ok(cursor.into_inner())
}
