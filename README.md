# 🦀 Fluffy Injector

<p align="center">
  <a href="https://github.com/fluffysnaff/fluffy-injector">
    <img src="https://github.com/fluffysnaff/fluffy-injector/raw/main/assets/icon.png" alt="Fluffy Injector Logo" width="150">
  </a>
</p>

**Fluffy Injector is an open-source Windows DLL injector written in Rust, with a native desktop interface and live process tracking. Explore more projects at [fluffysnaff.xyz](https://fluffysnaff.xyz) or on [GitHub](https://github.com/fluffysnaff).**

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

**Fluffy Injector** replaces command-line DLL injection workflows with a focused Windows desktop interface. It is designed for controlled development, research, and modification workflows:

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
- **🚀 Verified injection:** Uses Wraith-backed remote memory operations, Unicode paths, `LoadLibraryW`, and completion checks.
- **📋 Copy on inject:** Injects a temporary copy so the original DLL remains available for rebuilding, with an optional random filename.
- **💾 Persistent sessions:** Stores DLLs, checked selections, favorites, blocked process names, the last target name, window size, and multi-monitor placement in Windows AppData.
- **🎨 Native dark interface:** Uses `eframe` for a responsive Windows desktop experience.
- **🔔 Toast notifications:** Reports successful injections, warnings, and failures without blocking the interface.

---

## 🚀 Getting Started

### For Users (Recommended)

1. **Download:** Download and extract the `release-build` artifact from the latest successful [GitHub Actions build](https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml), or download `fluffy_injector.exe` from a published [GitHub Release](https://github.com/fluffysnaff/fluffy-injector/releases).
2. **Run:** Launch `fluffy_injector.exe`. No installation is required.
3. **Inject:**
   - Select a target process from the left panel.
   - Optionally right-click a process to **Favorite** it or **Block** it from the list.
   - To restore a blocked process, right-click empty space in the process list and choose it under **Blocked**.
   - Select **Add DLL** for each DLL you want to add.
   - Check every DLL you want to inject.
   - Optionally enable **Copy on inject** and **Random name**.
   - Select **Inject**.

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

The executable is written to `target\release\fluffy_injector.exe`.

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
