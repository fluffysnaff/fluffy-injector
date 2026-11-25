# Command line

```text
fluffy_injector [options] <pid|process> <dll>...
```

With no arguments, `fluffy_injector.exe` opens the desktop app. If the app is already open, its window comes to the front. With arguments, it runs in the calling terminal, prints its output there, and returns an exit code. It never creates a console window.

The command works from a new terminal after an install that adds it to PATH. Otherwise, use the full path to `fluffy_injector.exe`.

## Wait for the result

Fluffy Injector is a Windows GUI program. PowerShell and an interactive Command Prompt do not wait for GUI programs, so output can appear after the prompt and the exit code is not set.

In PowerShell, pipe the output. PowerShell then waits and sets `$LASTEXITCODE`:

```powershell
fluffy_injector --copy notepad.exe C:\hooks.dll | Out-Host
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

At an interactive Command Prompt, use `start /wait`:

```bat
start "" /wait /b fluffy_injector --copy notepad.exe C:\hooks.dll
```

Batch files (`.cmd` and `.bat`) wait without any change and set `%ERRORLEVEL%`.

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

## Temporary copies

`--copy` and `--random` place the copy in `%TEMP%luffy-injector`. After injection, Fluffy Injector tries to delete it. Windows does not allow this while the target has the copy loaded, so a successful injection normally also prints `warning: Could not remove temp DLL`. The warning does not change the exit code. You can delete the copy after the target closes.

If the load does not finish within 30 seconds, the command fails and keeps the copy, because the target can still be loading it.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Every DLL is loaded, or was already loaded. Also returned by `--help`. |
| `1` | Injection failed, or a DLL file is missing. |
| `2` | Bad arguments, process not found, or several processes share the name. |

When several processes share a name, the command lists their PIDs. Run it again with one of them.

A DLL whose file name is already loaded in the target counts as success and is not loaded again. With `--copy` or `--random`, the file name differs, so the DLL is loaded as a new image.
