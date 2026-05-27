pub fn read_password() -> Result<String, String> {
    rpassword::read_password().map_err(|e| format!("Failed to read password: {}", e))
}
