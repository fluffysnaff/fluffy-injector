fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_CFG_TARGET_OS")? != "windows" {
        return Ok(());
    }

    winresource::WindowsResource::new()
        .set_icon("assets/icon.ico")
        .set("ProductName", "Fluffy Injector")
        .set("FileDescription", "Fluffy Injector")
        .set("LegalCopyright", "Copyright (c) fluffysnaff")
        .compile()?;
    Ok(())
}
