# Build and install

## Build from source

You need Windows with the MSVC C++ build tools, [Rust nightly](https://rustup.rs/), and Git.

```powershell
git clone https://github.com/fluffysnaff/fluffy-injector.git
cd fluffy-injector
rustup toolchain install nightly
cargo +nightly build --release
```

The executable is `target\release\fluffy_injector.exe`.

Run the tests with:

```powershell
cargo +nightly test
```

## Install with the script

From a source checkout, build and install in one step:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\installer\Install-FluffyInjector.ps1 -Build
```

Leave out `-Build` to install a binary you already built. The script installs to `%LOCALAPPDATA%\Programs\Fluffy Injector`, adds that folder to your user PATH, creates a Start Menu shortcut, and registers an entry in **Apps & features**. Add `-Scope Machine` to install for all users.

To uninstall, use **Apps & features** or run the script with `-Uninstall`.

The portable `release-build` package from [GitHub Actions](https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml) contains the same script next to the executable.

## Settings

The desktop app saves its settings in `%APPDATA%luffy-injector\config.ron`. If that file does not exist, the app imports the settings from version 0.4 or earlier, `%APPDATA%\Fluffy Injector\datapp.ron`, and leaves the old file unchanged.

If `config.ron` cannot be read as valid settings, the app starts with defaults and does not save during that session. It leaves the file in place and copies it to `config.ron.bak`, unless a backup already exists. To start fresh, close the app and delete `config.ron`. If the old `app.ron` still exists, the app imports it again.

## Releases

Every build uploads the `release-build` package. A new `version` in `Cargo.toml` on `main` also publishes a [GitHub Release](https://github.com/fluffysnaff/fluffy-injector/releases) with the portable executable and the setup program, which is compiled from `installer\fluffy-injector.iss` with Inno Setup 6.
