use qrcode::QrCode;

/// Renders a string as a QR code in the terminal using Unicode block characters.
///
/// Each QR module is printed as a full-width character (█ for dark, space for light).
/// The result is readable by any QR scanner when the terminal has a light-coloured
/// background, or when the phone camera is pointed at it in a darkened room.
///
/// For maximum scannability, consider enabling a white background in your terminal
/// preferences before scanning.
pub fn print_qr(data: &str) -> Result<(), String> {
    let code = QrCode::new(data.as_bytes())
        .map_err(|e| format!("QR generation failed: {}", e))?;

    let image = code
        .render::<char>()
        .dark_color('█')
        .light_color(' ')
        .quiet_zone(true)
        .build();

    println!();
    for line in image.lines() {
        println!("    {}", line);
    }
    println!();
    Ok(())
}
