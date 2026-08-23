param(
    [ValidateSet('User', 'Machine')]
    [string]$Scope = 'User',
    [string]$Source,
    [switch]$Build,
    [switch]$Uninstall,
    [switch]$NoPath,
    [switch]$NoStartMenu,
    [switch]$NoPause
)

$ErrorActionPreference = 'Stop'

$AppName = 'Fluffy Injector'
$AppId = 'FluffyInjector'
$ExeName = 'fluffy_injector.exe'
$Publisher = 'fluffysnaff'
$ProjectUrl = 'https://github.com/fluffysnaff/fluffy-injector'
$PowerShellExe = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"

function Exit-Script([int]$Code) {
    if (-not $NoPause) { Write-Host ''; Read-Host 'Press Enter to exit' }
    exit $Code
}

function Test-IsAdministrator {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    ([Security.Principal.WindowsPrincipal]$id).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-RelaunchArgumentList {
    $argList = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Scope $Scope"
    if ($Source) { $argList += " -Source `"$Source`"" }
    if ($Build) { $argList += ' -Build' }
    if ($Uninstall) { $argList += ' -Uninstall' }
    if ($NoPath) { $argList += ' -NoPath' }
    if ($NoStartMenu) { $argList += ' -NoStartMenu' }
    if ($NoPause) { $argList += ' -NoPause' }
    $argList
}

function Request-Administrator {
    if (Test-IsAdministrator) { return }
    Write-Host '[*] Elevating for machine-wide install'
    Start-Process $PowerShellExe -Verb RunAs -ArgumentList (Get-RelaunchArgumentList)
    exit 0
}

function Get-ProductRoot {
    if ($Scope -eq 'Machine') { return (Join-Path $env:ProgramFiles $AppName) }
    Join-Path $env:LOCALAPPDATA "Programs\$AppName"
}

function Get-StartMenuFolder {
    if ($Scope -eq 'Machine') {
        return (Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs\$AppName")
    }
    Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$AppName"
}

function Get-UninstallKey {
    $hive = if ($Scope -eq 'Machine') { 'HKLM:' } else { 'HKCU:' }
    Join-Path $hive "Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppId"
}

function Get-RepoRoot {
    $root = Split-Path $PSScriptRoot -Parent
    if (Test-Path (Join-Path $root 'Cargo.toml')) { return $root }
}

function Get-PathEntries([string]$Target) {
    $raw = [Environment]::GetEnvironmentVariable('Path', $Target)
    if (-not $raw) { return @() }
    @($raw.Split(';') | Where-Object { $_ })
}

function Test-PathContains([string]$Directory, [string]$Target) {
    $needle = $Directory.TrimEnd('\')
    $null -ne (Get-PathEntries $Target | Where-Object { $_.TrimEnd('\') -ieq $needle })
}

function Set-PathEntries([string]$Target, [string[]]$Entries) {
    [Environment]::SetEnvironmentVariable('Path', ($Entries -join ';'), $Target)
}

function Add-AppPath([string]$Directory) {
    if (Test-PathContains $Directory $Scope) { return }
    Write-Host "[*] Adding $Directory to $Scope PATH"
    Set-PathEntries $Scope (@(Get-PathEntries $Scope) + $Directory)
}

function Remove-AppPath([string]$Directory) {
    if (-not (Test-PathContains $Directory $Scope)) { return }
    Write-Host "[*] Removing $Directory from $Scope PATH"
    $needle = $Directory.TrimEnd('\')
    Set-PathEntries $Scope @(Get-PathEntries $Scope | Where-Object { $_.TrimEnd('\') -ine $needle })
}

function Publish-Environment {
    $signature = @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
    uint flags, uint timeout, out UIntPtr result);
'@
    $type = 'Win32.EnvBroadcast' -as [type]
    if (-not $type) {
        $type = Add-Type -MemberDefinition $signature -Name EnvBroadcast -Namespace Win32 -PassThru
    }
    $result = [UIntPtr]::Zero
    [void]$type::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' +
        [Environment]::GetEnvironmentVariable('Path', 'User')
}

function Invoke-ReleaseBuild([string]$RepoRoot) {
    Write-Host '[*] cargo +nightly build --release'
    $cargo = Get-Command cargo.exe -ErrorAction SilentlyContinue
    if (-not $cargo) { throw 'cargo.exe not found on PATH' }
    Push-Location $RepoRoot
    try {
        & cargo +nightly build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
    }
    finally { Pop-Location }
}

function Resolve-ExistingExe([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { throw "Source exe not found: $Path" }
    (Resolve-Path -LiteralPath $Path).Path
}

function Resolve-RepoReleaseExe([string]$RepoRoot) {
    $built = Join-Path $RepoRoot "target\release\$ExeName"
    if ($Build -or -not (Test-Path -LiteralPath $built)) { Invoke-ReleaseBuild $RepoRoot }
    Resolve-ExistingExe $built
}

function Resolve-SourceExe {
    if ($Source) { return Resolve-ExistingExe $Source }
    $repo = Get-RepoRoot
    if ($Build -and $repo) { return Resolve-RepoReleaseExe $repo }
    $beside = Join-Path $PSScriptRoot $ExeName
    if (Test-Path -LiteralPath $beside) { return (Resolve-Path -LiteralPath $beside).Path }
    if ($repo) { return Resolve-RepoReleaseExe $repo }
    throw "No $ExeName found. Pass -Source or run from the repo."
}

function Get-AppVersion([string]$ExePath) {
    $fromExe = (Get-Item -LiteralPath $ExePath).VersionInfo.ProductVersion
    if ($fromExe -and $fromExe -ne '0.0.0.0') { return $fromExe }
    $root = Get-RepoRoot
    if (-not $root) { return '0.3.0' }
    $line = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^\s*version\s*=\s*"([^"]+)"' |
        Select-Object -First 1
    if ($line) { return $line.Matches[0].Groups[1].Value }
    '0.3.0'
}

function Resolve-LicensePath([string]$ExePath) {
    $roots = @($PSScriptRoot, (Split-Path $ExePath -Parent))
    $repo = Get-RepoRoot
    if ($repo) { $roots = @($repo) + $roots }
    foreach ($root in $roots) {
        $license = Join-Path $root 'LICENSE'
        if (Test-Path -LiteralPath $license) { return $license }
    }
}

function Copy-InstallFiles([string]$ExePath, [string]$Root) {
    Write-Host "[*] Installing files to $Root"
    New-Item -ItemType Directory -Force -Path $Root | Out-Null
    Copy-Item -LiteralPath $ExePath -Destination (Join-Path $Root $ExeName) -Force
    Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $Root (Split-Path $PSCommandPath -Leaf)) -Force
    $license = Resolve-LicensePath $ExePath
    if ($license) {
        Copy-Item -LiteralPath $license -Destination (Join-Path $Root 'LICENSE') -Force
    }
}

function Install-StartMenuShortcut([string]$ExePath) {
    $folder = Get-StartMenuFolder
    New-Item -ItemType Directory -Force -Path $folder | Out-Null
    $link = Join-Path $folder "$AppName.lnk"
    Write-Host "[*] Start Menu shortcut $link"
    $shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($link)
    $shortcut.TargetPath = $ExePath
    $shortcut.WorkingDirectory = Split-Path $ExePath -Parent
    $shortcut.IconLocation = "$ExePath,0"
    $shortcut.Description = $AppName
    $shortcut.Save()
}

function Uninstall-StartMenuShortcut {
    $folder = Get-StartMenuFolder
    if (Test-Path -LiteralPath $folder) {
        Write-Host "[*] Removing Start Menu folder $folder"
        Remove-Item -LiteralPath $folder -Recurse -Force
    }
}

function Set-UninstallValue([string]$Key, [string]$Name, $Value, [string]$Type) {
    New-ItemProperty -Path $Key -Name $Name -PropertyType $Type -Value $Value -Force | Out-Null
}

function Register-AddRemovePrograms([string]$Root, [string]$ExePath, [string]$Version) {
    $key = Get-UninstallKey
    $script = Join-Path $Root (Split-Path $PSCommandPath -Leaf)
    $uninstall = "`"$PowerShellExe`" -NoProfile -ExecutionPolicy Bypass -File `"$script`" -Uninstall -Scope $Scope -NoPause"
    Write-Host '[*] Registering Add/Remove Programs entry'
    New-Item -Path $key -Force | Out-Null
    Set-UninstallValue $key DisplayName $AppName String
    Set-UninstallValue $key DisplayVersion $Version String
    Set-UninstallValue $key Publisher $Publisher String
    Set-UninstallValue $key InstallLocation $Root String
    Set-UninstallValue $key DisplayIcon $ExePath String
    Set-UninstallValue $key UninstallString $uninstall String
    Set-UninstallValue $key QuietUninstallString $uninstall String
    Set-UninstallValue $key HelpLink $ProjectUrl String
    Set-UninstallValue $key URLInfoAbout $ProjectUrl String
    Set-UninstallValue $key NoModify 1 DWord
    Set-UninstallValue $key NoRepair 1 DWord
    Set-UninstallValue $key EstimatedSize ([int]((Get-Item -LiteralPath $ExePath).Length / 1KB)) DWord
}

function Unregister-AddRemovePrograms {
    $key = Get-UninstallKey
    if (Test-Path -LiteralPath $key) {
        Write-Host '[*] Removing Add/Remove Programs entry'
        Remove-Item -LiteralPath $key -Recurse -Force
    }
}

function Install-Application {
    $root = Get-ProductRoot
    $exe = Join-Path $root $ExeName
    Copy-InstallFiles (Resolve-SourceExe) $root
    if (-not $NoStartMenu) { Install-StartMenuShortcut $exe }
    if (-not $NoPath) { Add-AppPath $root }
    Register-AddRemovePrograms $root $exe (Get-AppVersion $exe)
    Publish-Environment
    Write-Host "[OK] $AppName $($Scope.ToLower()) install ready at $root"
    Write-Host "    GUI: Start Menu or $exe"
    Write-Host "    CLI: $ExeName --help"
}

function Uninstall-Application {
    $root = Get-ProductRoot
    if (-not (Test-Path -LiteralPath $root)) {
        Write-Host "[WARN] $AppName is not installed for scope $Scope"
        return
    }
    Uninstall-StartMenuShortcut
    Remove-AppPath $root
    Unregister-AddRemovePrograms
    Write-Host "[*] Removing $root"
    Set-Location $env:TEMP
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    Publish-Environment
    Write-Host "[OK] $AppName removed"
}

try {
    if ($Scope -eq 'Machine') { Request-Administrator }
    if ($Uninstall) { Uninstall-Application } else { Install-Application }
    Exit-Script 0
}
catch {
    Write-Host "[ERROR] $_"
    Exit-Script 1
}
