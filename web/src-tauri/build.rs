use std::path::Path;

fn main() {
    // Create a minimal icon.ico if it doesn't exist (for dev builds)
    let icon_path = Path::new("icons/icon.ico");
    if !icon_path.exists() {
        std::fs::create_dir_all("icons").ok();
        // Create a minimal valid ICO file (16x16 black square)
        // ICO header + one image
        let ico_data = vec![
            0, 0, 1, 0, // ICO magic + reserved + type
            1, 0, // number of images
            16, 16, // width, height
            0, // colors in palette
            0, // reserved
            1, 0, // color planes
            32, 0, // bits per pixel
            0x44, 1, 0, 0, // size of image data (324 bytes)
            22, 0, 0, 0, // offset to image data
        ];

        // BMP header
        let mut bmp_header = vec![
            40, 0, 0, 0, // header size
            16, 0, 0, 0, // width
            32, 0, 0, 0, // height (doubled for ICO)
            1, 0, // planes
            32, 0, // bits per pixel
            0, 0, 0, 0, // compression
            0, 0, 0, 0, // image size
            0, 0, 0, 0, // x pixels per meter
            0, 0, 0, 0, // y pixels per meter
            0, 0, 0, 0, // colors used
            0, 0, 0, 0, // important colors
        ];

        // Simple black square pixel data (BGRA format)
        let pixels = vec![0u8; 16 * 16 * 4]; // all black pixels
        let mask = vec![0u8; 16 * 4]; // AND mask (all transparent)

        let mut full_ico = ico_data;
        full_ico.append(&mut bmp_header);
        full_ico.extend_from_slice(&pixels);
        full_ico.extend_from_slice(&mask);

        std::fs::write(icon_path, full_ico).ok();
    }

    tauri_build::build()
}
