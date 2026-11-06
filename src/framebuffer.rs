use std::fs;
use std::path::Path;

use raylib::prelude::*;

/// CPU color (RGBA8) and depth buffers. Depth stores NDC z; smaller values
/// are closer. `clear` restores a fixed background image.
pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    pub color: Vec<u8>,
    pub depth: Vec<f32>,
    background: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: i32, height: i32, background_color: Color) -> Self {
        let pixels = (width * height) as usize;
        let background = [
            background_color.r,
            background_color.g,
            background_color.b,
            255,
        ]
        .repeat(pixels);
        Framebuffer {
            width,
            height,
            color: background.clone(),
            depth: vec![f32::INFINITY; pixels],
            background,
        }
    }

    /// Replaces the background with an RGBA8 image of the same size.
    pub fn set_background(&mut self, rgba: Vec<u8>) {
        assert_eq!(rgba.len(), self.color.len());
        self.background = rgba;
    }

    pub fn clear(&mut self) {
        self.color.copy_from_slice(&self.background);
        self.depth.fill(f32::INFINITY);
    }

    pub fn get_color(&self, x: i32, y: i32) -> Color {
        let i = ((y * self.width + x) * 4) as usize;
        Color::new(self.color[i], self.color[i + 1], self.color[i + 2], 255)
    }

    pub fn get_depth(&self, x: i32, y: i32) -> f32 {
        self.depth[(y * self.width + x) as usize]
    }

    /// Saves the buffer as a PNG, creating the parent directory if needed.
    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("cannot create {}: {err}", parent.display()))?;
        }
        let mut image = Image::gen_image_color(self.width, self.height, Color::BLACK);
        for y in 0..self.height {
            for x in 0..self.width {
                image.draw_pixel(x, y, self.get_color(x, y));
            }
        }
        // raylib's ExportImage does not report failures, so remove any old
        // file first and check that a new one was written.
        let _ = fs::remove_file(path);
        let path_str = path
            .to_str()
            .ok_or_else(|| format!("non UTF-8 path {}", path.display()))?;
        image.export_image(path_str);
        if path.is_file() {
            Ok(())
        } else {
            Err(format!("raylib could not write {}", path.display()))
        }
    }
}
