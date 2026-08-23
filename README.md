# 🦀 Fluffy Injector

<p align="center">
  <a href="https://github.com/fluffysnaff/fluffy-injector">
    <img src="https://github.com/fluffysnaff/fluffy-injector/raw/main/assets/icon.png" alt="Fluffy Injector Logo" width="150">
  </a>
</p>

**Fluffy Injector is an open-source Windows DLL injector written in Rust, with a native desktop interface, live process tracking, and a headless CLI. Explore more projects at [fluffysnaff.xyz](https://fluffysnaff.xyz) or on [GitHub](https://github.com/fluffysnaff).**

<p align="center">
    <a href="https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml"><img src="https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml/badge.svg" alt="Build Status"></a>
    <a href="https://github.com/fluffysnaff/fluffy-injector/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
    <img src="https://img.shields.io/badge/Made%20with-Rust-orange.svg" alt="Made with Rust">
    <img src="https://img.shields.io/badge/Platform-Windows-0078D6.svg" alt="Platform: Windows">
</p>

---

> ⚠️ **Disclaimer: For Educational Use Only**
>
> This tool is intended strictly for **educational and development purposes**, such as working with software you own or are authorized to inspect. Injecting DLLs into arbitrary processes can cause **application crashes, system instability, or anti-cheat penalties**.
>
> **Use this tool responsibly and ethically.** The author (`fluffysnaff`) is not responsible for any damage or consequences resulting from its misuse.

---

## 🖼️ GUI Preview

<p align="center">
  <img src="https://github.com/fluffysnaff/fluffy-injector/raw/main/assets/screenshot.png" alt="Fluffy Injector UI Screenshot">
</p>

---

## What is Fluffy Injector?

**Fluffy Injector** is a focused Windows desktop injector with an optional headless CLI for scripts and post-build steps. It is designed for controlled development, research, and modification workflows:

- **Developers:** Load custom DLLs while developing software.
- **Security researchers:** Analyze process behavior and interactions.
- **Modders and enthusiasts:** Experiment with authorized application modifications.

The goal is to make DLL injection straightforward without hiding important controls or results.

---

## ✨ Key Features

- **🔍 Native process scanning:** Lists running processes with their names, PIDs, and available application icons.
- **⚡ Live filtering:** Searches the process list as you type.
- **🔄 Automatic process tracking:** Removes terminated processes and reacquires a same-name replacement without a manual refresh.
- **⭐ Favorites:** Right-click a process to favorite or unfavorite it. Favorites stay pinned at the top of the list.
- **🚫 Block list:** Right-click a process to hide it from the list. Right-click empty space in the process panel, open **Blocked**, and select a name to unblock it.
- **📂 Multi-DLL management:** Adds, selects, injects, and removes one or more DLLs from a persistent list.
- **📎 DLL context menu:** Right-click a DLL to open its file location, inject it into the selected process, or remove it. Right-click empty space in the DLL list to add a DLL.
- **⌨️ Headless CLI:** Injects from a terminal with a PID or process name plus one or more DLL paths. `--copy` and `--random` match the GUI copy-on-inject options. No arguments still launches the GUI.
- **🚀 Verified injection:** Uses Wraith-backed remote memory operations, Unicode paths, `LoadLibraryW`, and completion checks. Already-mapped modules are left in place instead of calling `LoadLibraryW` again.
- **📋 Copy on inject:** Injects a temporary copy so the original DLL remains available for rebuilding, with an optional random filename.
- **💾 Persistent sessions:** Stores DLLs, checked selections, favorites, blocked process names, the last target name, split ratio, window size, and multi-monitor placement in Windows AppData.
- **🎨 Native dark interface:** Uses `eframe` for a responsive Windows desktop experience, including a resizable process / DLL split.
- **🔔 Toast notifications:** Reports successful injections, warnings, and failures without blocking the interface.

---

## 🚀 Getting Started

### For Users (Recommended)

1. **Download:** Prefer a [GitHub Release](https://github.com/fluffysnaff/fluffy-injector/releases) (`FluffyInjector-*-setup.exe` and `fluffy_injector.exe`). A new `version` in `Cargo.toml` on `main` publishes that Release. Every build also uploads a portable `release-build` artifact from [GitHub Actions](https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml).
2. **Install (recommended):** Run the setup exe, or from the extracted zip:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\Install-FluffyInjector.ps1
   ```

   Per-user is the default (`%LOCALAPPDATA%\Programs\Fluffy Injector`). It adds that folder to your user PATH, creates a Start Menu shortcut, and registers **Apps & features** uninstall. Machine-wide:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\Install-FluffyInjector.ps1 -Scope Machine
   ```

   From a source checkout, `-Build` compiles a release binary first, then installs it. Uninstall from **Apps & features**, or:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File .\Install-FluffyInjector.ps1 -Uninstall
   ```

3. **Or run portable:** Launch `fluffy_injector.exe` with no install. The CLI will not be on PATH.
4. **Inject:**
   - Select a target process from the left panel.
   - Optionally right-click a process to **Favorite** it or **Block** it from the list.
   - To restore a blocked process, right-click empty space in the process list and choose it under **Blocked**.
   - Select **Add DLL** for each DLL you want to add, or right-click empty space in the DLL list → **Add DLL**.
   - Check every DLL you want to inject, or right-click a DLL for **Open file location**, **Inject**, or **Delete**.
   - Optionally enable **Copy on inject** and **Random name**.
   - Select **Inject** (or use **Inject** from a DLL’s context menu for a single file).

### For Developers (Building from Source)

Requirements:

- Windows with the MSVC C++ build tools
- [Rust nightly](https://rustup.rs/)
- Git

```powershell
git clone https://github.com/fluffysnaff/fluffy-injector.git
cd fluffy-injector
rustup toolchain install nightly
cargo +nightly build --release
```

The executable is written to `target\release\fluffy_injector.exe`. To install that build onto PATH and the Start Menu:

```powershell
.\installer\Install-FluffyInjector.ps1
```

Or `.\installer\Install-FluffyInjector.ps1 -Build` to compile and install in one step. CI also compiles `installer\fluffy-injector.iss` with Inno Setup 6 into `FluffyInjector-<version>-setup.exe`.

### Headless CLI

Launching `fluffy_injector.exe` with no arguments still opens the GUI and hides the console. Any other argument stays in the terminal so scripts and post-build steps get stdout, stderr, and the exit code. Opening the GUI from Explorer can flash a console window briefly.

After a PATH install, `fluffy_injector` works from a new terminal. Portable or repo builds need the full exe path.

```powershell
fluffy_injector notepad.exe C:\hooks.dll
fluffy_injector --copy --random Gw2-64.exe C:\hooks.dll
fluffy_injector 1234 C:\hooks.dll C:\overlay.dll
```

Options may appear before or after the process name. The first non-option argument is a PID if it is all digits, otherwise a case-insensitive process name (`.exe` optional). Every later non-option is a DLL path, resolved against the injector working directory rather than the target process CWD.

| Option | Meaning |
| --- | --- |
| `-c` / `--copy` | Inject a temp copy so the original DLL stays free to rebuild. |
| `-r` / `--random` | Give that copy a random file name. Implies `--copy`. |
| `-n` / `--name` / `-p` / `--pid` | Aliases for the process name or PID. |
| `-h` / `--help` | Print usage and exit `0`. |

Eject and window-title targeting stay GUI-only. If several processes share the same name, the CLI lists their PIDs and exits `2` so you can pass a PID. A DLL whose file name is already mapped in the target is treated as success and is not loaded again. `--copy` / `--random` still load a new image because the mapped name differs.

| Exit code | Meaning |
| --- | --- |
| `0` | GUI not involved; every requested DLL is mapped (injected or already present). `--help` also exits `0`. |
| `1` | Injection failed, including a missing DLL file. |
| `2` | Bad arguments, process not found, or an ambiguous process name. |

---

## 🛠️ Technology Stack

- **[eframe](https://crates.io/crates/eframe):** Desktop application framework and persistent window state.
- **[wraith-rs](https://crates.io/crates/wraith-rs):** Remote process access, memory operations, module discovery, and allocation cleanup.
- **[windows-rs](https://github.com/microsoft/windows-rs):** Windows process snapshots, thread creation, executable icons, and native handles.
- **[rfd](https://github.com/PolyMeilex/rfd):** Native DLL file selection.
- **[Serde](https://serde.rs/) and [RON](https://github.com/ron-rs/ron):** Persistent application settings.

---

## 🤝 Contributing

Contributions are welcome! Whether you have ideas for new features, bug fixes, or code improvements, your help is appreciated.

- **Report issues:** Open an issue on the [GitHub Issues page](https://github.com/fluffysnaff/fluffy-injector/issues).
- **Suggest features:** Start a discussion by creating an issue.
- **Submit pull requests:** Open an issue first to discuss the proposed change and confirm it fits the project.

---

## 📜 License

This project is licensed under the **MIT License**. See the [LICENSE](https://github.com/fluffysnaff/fluffy-injector/blob/main/LICENSE) file for full details.

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=fluffysnaff/fluffy-injector&type=Date)](https://www.star-history.com/#fluffysnaff/fluffy-injector&Date)
