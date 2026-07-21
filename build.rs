fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_CFG_TARGET_OS")? == "windows" {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()?;
    }
    Ok(())
}
