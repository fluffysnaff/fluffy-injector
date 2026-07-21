# 🦀 Fluffy Injector

<p align="center">
  <a href="https://github.com/fluffysnaff/fluffy-injector">
    <img src="https://github.com/fluffysnaff/fluffy-injector/raw/main/assets/icon.png" alt="Fluffy Injector Logo" width="150">
  </a>
</p>

**A modern, open-source `Rust DLL injector` for Windows, featuring a sleek `egui GUI` and powerful process management capabilities. Find more projects at [fluffysnaff.xyz](https://fluffysnaff.xyz) and on MY [GitHub](https://github.com/fluffysnaff).**

<p align="center">
    <a href="https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml"><img src="https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml/badge.svg" alt="Build Status"></a>
    <a href="https://github.com/fluffysnaff/fluffy-injector/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
    <img src="https://img.shields.io/badge/Made%20with-Rust-orange.svg" alt="Made with Rust">
    <img src="https://img.shields.io/badge/Platform-Windows-0078D6.svg" alt="Platform: Windows">
</p>

---

> ⚠️ **Disclaimer: For Educational Use Only**
>
> This `Windows injector tool` is intended strictly for **educational and development purposes**, such as testing your own software. Injecting DLLs into arbitrary processes can lead to **application crashes, system instability, or detection and banning by anti-cheat software**.
>
> **Use this tool responsibly and ethically.** The author (`fluffysnaff`) is not responsible for any damage or consequences resulting from its misuse.

---

## 🖼️ GUI Preview

<p align="center">
  <img src="https://github.com/fluffysnaff/fluffy-injector/raw/main/assets/screenshot.png" alt="Fluffy Injector UI Screenshot">
</p>

---

## What is Fluffy Injector?

**Fluffy Injector** is a `Windows injector tool` that replaces complex command-line utilities with a clean, intuitive graphical interface. Built entirely in Rust, it provides a safe and efficient way to perform `DLL injection`. It's the perfect utility for:

*   **Developers:** Quickly test your custom DLLs in a live environment.
*   **Security Researchers:** Analyze process behavior and interactions.
*   **Modders & Enthusiasts:** Experiment with game or application modifications in a controlled way.

The goal is to make `DLL injection` accessible and straightforward without sacrificing control.

---

## ✨ Key Features

*   **🔍 Smart Process Scanning:** Lists all running processes with their name, PID, and application icon for easy identification.
*   **⚡ Real-time Filtering:** Instantly search the process list to find exactly what you're looking for.
*   **🔄 Live Process Tracking:** Uses lightweight native Windows snapshots to remove terminated processes and reacquire same-name replacements automatically.
*   **📂 Easy DLL Management:** Add DLLs, select one or more with checkboxes, and manage them in a persistent list.
*   **🚀 One-Click Injection:** Injects every selected DLL with Wraith-backed remote process operations, Unicode paths, verified `LoadLibraryW` completion, and automatic remote-memory cleanup.
*   **📋 Copy on Inject:** Optionally inject a temporary DLL copy so the original build output remains free for rebuilding, with an optional random filename.
*   **💾 Session Persistence:** Remembers your DLL list, checked DLLs, last selected application, window size, and multi-monitor placement.
*   **🎨 Modern Dark UI:** Built with Rust's immediate-mode `egui GUI` framework for a responsive, cross-platform feel.
*   **🔔 Toast Notifications:** Get instant, non-intrusive feedback on injection success, warnings, or failures.

---

## 🚀 Getting Started

### For Users (Recommended)

1.  **Download:** Grab `release-build` from the latest successful [**GitHub Actions build**](https://github.com/fluffysnaff/fluffy-injector/actions/workflows/rust.yml) or download `fluffy_injector.exe` from a published [**GitHub Release**](https://github.com/fluffysnaff/fluffy-injector/releases).
2.  **Run:** Place the executable in a folder of your choice and run it. No installation is required.
3.  **Inject:**
    *   Select a target process from the left panel.
    *   Click "Add DLL" to add your desired DLL to the list in the right panel.
    *   Select the DLL you just added.
    *   Click "Inject"!

### For Developers (Building from Source)

1.  **Requirements:**
    *   [Rust Toolchain](https://rustup.rs/) (nightly)
    *   Git
2.  **Clone the Repository:**
    ```bash
    git clone https://github.com/fluffysnaff/fluffy-injector.git
    cd fluffy-injector
    ```
3.  **Build and Run:**
    ```bash
    # For a debug build
    cargo run
    
    # For a release build (recommended for performance)
    cargo run --release 
    ```
    The final executable will be located in the `target/release` directory.

---

## 🛠️ Technology Stack

Fluffy Injector is built with a modern Rust ecosystem:

*   **[egui](https://github.com/emilk/egui) & [eframe](https://github.com/emilk/egui/tree/master/crates/eframe):** For the immediate-mode graphical user interface.
*   **[wraith-rs](https://crates.io/crates/wraith-rs):** For remote process access, direct-syscall memory operations, module discovery, and RAII cleanup.
*   **[windows-rs](https://github.com/microsoft/windows-rs):** For safe, idiomatic bindings to Windows APIs required for process tracking, thread creation, and icon extraction.
*   **[rfd](https://github.com/PolyMeilex/rfd):** For native, platform-appropriate "open file" dialogs.
*   **[Serde](https://serde.rs/):** For robust serialization and deserialization of persisted application settings.

---

## 🤝 Contributing

Contributions are welcome! Whether you have ideas for new features, bug fixes, or code improvements, your help is appreciated.

*   **Report Issues:** Found a bug? Open an issue on the [**GitHub Issues page**](https://github.com/fluffysnaff/fluffy-injector/issues).
*   **Suggest Features:** Have an idea? Start a discussion by creating an issue.
*   **Submit Pull Requests:** Please open an issue first to discuss your planned changes. This helps ensure your work aligns with the project's goals.

---

## 📜 License

This project is licensed under the **MIT License**. See the [LICENSE](https://github.com/fluffysnaff/fluffy-injector/blob/main/LICENSE) file for full details.

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=fluffysnaff/fluffy-injector&type=Date)](https://www.star-history.com/#fluffysnaff/fluffy-injector&Date)
