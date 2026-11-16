$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'src-tauri/target/i686-pc-windows-msvc/release/ds-workshop-tool.exe'
$dll = Join-Path $root 'src-tauri/resources/steam_api.dll'
if (-not (Test-Path $exe)) { throw "Release exe not found. Run npm run desktop:build first." }
if (-not (Test-Path $dll)) { throw "x86 steam_api.dll not found." }
$out = Join-Path $root '.local/portable/Workshop Studio'
Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item $out -ItemType Directory -Force | Out-Null
New-Item (Join-Path $out 'resources') -ItemType Directory -Force | Out-Null
Copy-Item $exe (Join-Path $out 'Workshop Studio.exe')
Copy-Item $dll (Join-Path $out 'resources/steam_api.dll')
Write-Host "Portable directory: $out"
Get-ChildItem $out -Recurse | Select-Object FullName, Length
