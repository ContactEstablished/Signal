# Dot-source before native development on machines with multiple Visual Studio versions.
# Changes only this PowerShell session. Prefers an installation with C++ tools.
$signalVsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$signalVsPath = & $signalVsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $signalVsPath) { throw 'Install the Visual Studio Desktop development with C++ workload.' }
& (Join-Path $signalVsPath 'Common7/Tools/Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Install Rust stable for x86_64-pc-windows-msvc.' }
