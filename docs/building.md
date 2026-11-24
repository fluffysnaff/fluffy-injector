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

## Install with the script

From a source checkout, build and install in one step:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\installer\Install-FluffyInjector.ps1 -Build
```

Leave out `-Build` to install a binary you already built. The script installs to `%LOCALAPPDATA%\Programs\Fluffy Injector`, adds that folder to your user PATH, creates a Start Menu shortcut, and registers an entry in **Apps & features**. Add `-Scope Machine` to install for all users.

To uninstall, use **Apps & features** or run the script with `-Uninstall`.

The portable `release-build` package from [GitHub Actions](https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml) contains the same script next to the executable.

## Releases

Every build uploads the `release-build` package. A new `version` in `Cargo.toml` on `main` also publishes a [GitHub Release](https://github.com/fluffysnaff/fluffy-injector/releases) with the portable executable and the setup program, which is compiled from `installer\fluffy-injector.iss` with Inno Setup 6.
