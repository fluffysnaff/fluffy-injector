# Command line

```text
fluffy_injector [options] <pid|process> <dll>...
```

With no arguments, `fluffy_injector.exe` opens the taut desktop app. It is a Windows GUI-subsystem executable and never creates a console window. With arguments, it runs the CLI, tries to attach to the calling terminal for output, and returns an exit code.

The command works from a new terminal after an install that adds it to PATH. Otherwise, use the full path to `fluffy_injector.exe`.

## Arguments

- **Process:** a process ID (PID) if it is all digits. Otherwise it is a process name. Case is ignored and `.exe` is optional.
- **DLLs:** one or more paths. Relative paths start from your current directory, not from the target's directory.

Options can come before or after the process.

| Option | Meaning |
| --- | --- |
| `-c`, `--copy` | Inject a temporary copy so the original DLL stays free to rebuild. |
| `-r`, `--random` | Give that copy a random file name. Implies `--copy`. |
| `-n`, `--name`, `-p`, `--pid` | Optional labels before the process name or PID. |
| `-h`, `--help` | Print usage and exit. Use it as the first argument. |

```powershell
fluffy_injector --copy notepad.exe C:\hooks.dll
```

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Every DLL is loaded, or was already loaded. Also returned by `--help`. |
| `1` | Injection failed, or a DLL file is missing. |
| `2` | Bad arguments, process not found, or several processes share the name. |

When several processes share a name, the command lists their PIDs. Run it again with one of them.

A DLL whose file name is already loaded in the target counts as success and is not loaded again. With `--copy` or `--random`, the file name differs, so the DLL is loaded as a new image.
