use std::path::Path;

fn main() {
    // Create a minimal icon.ico if it doesn't exist (for dev builds)
    let icon_path = Path::new("icons/icon.ico");
    if !icon_path.exists() {
        std::fs::create_dir_all("icons").ok();

        // Create a minimal valid 1x1 pixel ICO file
        // ICO format: Header (6 bytes) + Directory Entry (16 bytes) + PNG data

        // Simpler approach: use a 1x1 pixel PNG embedded in ICO format
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
            0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41,
            0x54, 0x78, 0x9C, 0x62, 0x00, 0x01, 0x00, 0x00,
            0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
            0x42, 0x60, 0x82, // IEND chunk
        ];

        let png_size = png_data.len() as u32;

        let mut ico_data = vec![
            0x00, 0x00, // Reserved (must be 0)
            0x01, 0x00, // Type (1 = ICO)
            0x01, 0x00, // Number of images
            // Directory entry
            0x01,       // Width (1 pixel)
            0x01,       // Height (1 pixel)
            0x00,       // Color palette size
            0x00,       // Reserved
            0x01, 0x00, // Color planes
            0x20, 0x00, // Bits per pixel
        ];

        // Image size (4 bytes, little-endian)
        ico_data.extend_from_slice(&png_size.to_le_bytes());

        // Image offset (4 bytes, little-endian) - starts after header (6) + directory (16) = 22
        ico_data.extend_from_slice(&22u32.to_le_bytes());

        // Append PNG data
        ico_data.extend_from_slice(&png_data);

        std::fs::write(icon_path, ico_data).ok();
    }

    tauri_build::build()
}
