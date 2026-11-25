# Changelog

Changes that affect people who use or build Fluffy Injector. Dates are release tag dates.

## 0.5.1 - 2026-11-25

### Fixed

- DLL names and checkboxes are visible again. In 0.5.0, a large icon covered each row.

## 0.5.0 - 2026-11-24

### Added

- Drag DLL files into the list. Use `Ctrl+O` to add a DLL and `Ctrl+F` to filter processes.
- Opening the app twice shows the window that is already open.
- When several processes share the remembered name, the app asks you to choose one.

### Changed

- The desktop app uses taut instead of eframe and never opens a console window.
- PowerShell and an interactive Command Prompt no longer wait for the command. See [how to wait for the result](docs/cli.md#wait-for-the-result).
- Settings moved to `%APPDATA%\fluffy-injector\config.ron`. Older settings are imported once, and invalid settings are backed up.

### Known issues

- DLL names and checkboxes are hidden in the desktop app. Fixed in 0.5.1.
- Copy on inject can warn that it could not remove the temporary DLL after a successful injection. See [temporary copies](docs/cli.md#temporary-copies). Also applies to 0.5.1.

## 0.4.2 - 2026-08-23

### Fixed

- The setup program can add the command to PATH when it installs for all users.

## 0.4.1 - 2026-08-23

### Added

- Command line for injection from a terminal or build script, with `--copy` and `--random`.
- Setup program and PowerShell install script.

### Fixed

- **Open file location** opens the DLL folder in Explorer.

## 0.4.0 - 2026-07-23

### Added

- Right-click menus in the DLL list to inject, delete, add, or open the file location.

## 0.3.0 - 2026-07-22

### Added

- Select and inject several DLLs at once.
- Copy on inject, with an optional random name.
- Live process list that follows a restarted process.
- Favorite and blocked processes.
- Saved window position and size.

### Changed

- New split layout with adjustable panels.
- The executable is now named `fluffy_injector.exe`.

## 0.2.0 - 2025-07-10

### Changed

- Reorganized the application code.

## 0.1.1 - 2025-02-28

### Added

- Process and DLL icons.
- Saved process and DLL selection.
- Remove button and duplicate DLL check.

## 0.1.0 - 2025-02-28

- First release.
