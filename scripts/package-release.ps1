# Generate a clean, allowlisted ZIP from the existing Release build. Never launches the app.
$ErrorActionPreference = 'Stop'
function FileHash([string]$path) {
    $stream = [IO.File]::OpenRead($path); $sha = [Security.Cryptography.SHA256]::Create()
    try { return -join ($sha.ComputeHash($stream) | ForEach-Object { $_.ToString('x2') }) }
    finally { $sha.Dispose(); $stream.Dispose() }
}
$root = Split-Path -Parent $PSScriptRoot
$config = Get-Content (Join-Path $root 'src-tauri/tauri.conf.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$package = Get-Content (Join-Path $root 'package.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$version = $config.version
if ($version -notmatch '^\d+\.\d+\.\d+([.-][a-zA-Z0-9.-]+)?$' -or $version -ne $package.version) { throw 'Invalid or mismatched release version.' }
if (@($config.app.windows | Where-Object { $_.additionalBrowserArgs }).Count -gt 0) { throw 'Do not package a debug browser configuration.' }
$exe = Join-Path $root 'src-tauri/target/i686-pc-windows-msvc/release/ds-workshop-tool.exe'
$dll = Join-Path $root 'src-tauri/resources/steam_api.dll'
# Keep this script ASCII-compatible with Windows PowerShell 5.1.
$readmeName = -join ([char[]]@(0x4f7f,0x7528,0x8bf4,0x660e))
$readmeName += '.txt'
$readme = Join-Path (Join-Path $root 'docs') $readmeName
foreach ($file in @($exe, $dll, $readme)) { if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Missing release input: $file" } }
foreach ($file in @($exe, $dll)) {
    $bytes = [IO.File]::ReadAllBytes($file)
    $pe = [BitConverter]::ToInt32($bytes, 60)
    if ([BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x14c) { throw "Expected x86 PE: $file" }
}
$out = Join-Path $root '.local/releases'
New-Item $out -ItemType Directory -Force | Out-Null
$stage = Join-Path $out ('.stage-' + [Guid]::NewGuid().ToString('N'))
$payload = Join-Path $stage 'Workshop Studio'
$zipName = "Workshop-Studio-$version-Windows-test.zip"
$zipPath = Join-Path $out $zipName
$checks = @{}
try {
    New-Item (Join-Path $payload 'resources') -ItemType Directory -Force | Out-Null
    Copy-Item -LiteralPath $exe -Destination (Join-Path $payload 'Workshop Studio.exe')
    Copy-Item -LiteralPath $dll -Destination (Join-Path $payload 'resources/steam_api.dll')
    $text = [IO.File]::ReadAllText($readme, [Text.Encoding]::UTF8)
    $text = "Workshop Studio v$version - Windows Test Build`r`n`r`n" + ($text -replace '\r?\n', "`r`n")
    [IO.File]::WriteAllText((Join-Path $payload $readmeName), $text, (New-Object Text.UTF8Encoding($true)))
    $allowlist = @('Workshop Studio/Workshop Studio.exe', 'Workshop Studio/resources/steam_api.dll', "Workshop Studio/$readmeName")
    foreach ($file in Get-ChildItem $payload -Recurse -File) {
        $relative = $file.FullName.Substring($stage.Length + 1).Replace('\', '/')
        if ($relative -notin $allowlist) { throw "Unexpected release payload: $relative" }
        $checks[$relative] = FileHash $file.FullName
    }
    if ($checks.Count -ne 3) { throw 'Release payload must contain exactly three files.' }
    $tempZip = Join-Path $stage $zipName
    Compress-Archive -LiteralPath $payload -DestinationPath $tempZip -CompressionLevel Optimal
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($tempZip)
    try {
        $entries = @($archive.Entries | Where-Object { $_.Name })
        if ($entries.Count -ne 3) { throw 'Unexpected ZIP file count.' }
        foreach ($entry in $entries) {
            $name = $entry.FullName.Replace('\', '/')
            if (-not $checks.ContainsKey($name)) { throw "Unexpected ZIP entry: $name" }
            $stream = $entry.Open(); $sha = [Security.Cryptography.SHA256]::Create()
            try { $hash = -join ($sha.ComputeHash($stream) | ForEach-Object { $_.ToString('x2') }) }
            finally { $sha.Dispose(); $stream.Dispose() }
            if ($hash -ne $checks[$name]) { throw "ZIP hash mismatch: $name" }
        }
    } finally { $archive.Dispose() }
    Copy-Item -LiteralPath $tempZip -Destination $zipPath -Force
    $zipHash = FileHash $zipPath
    $lines = @("$zipHash  $zipName")
    foreach ($name in ($checks.Keys | Sort-Object)) { $lines += "$($checks[$name])  $name" }
    [IO.File]::WriteAllText("$zipPath.sha256.txt", ($lines -join "`r`n") + "`r`n", (New-Object Text.UTF8Encoding($true)))
    Write-Host "Clean release ZIP: $zipPath"
    Write-Host "SHA256: $zipHash"
    Write-Host 'Verified exactly 3 files, all extracted byte hashes match; no logs, drafts or diagnostic copies.'
    Get-Item -LiteralPath $zipPath, "$zipPath.sha256.txt" | Select-Object FullName, Length
} finally {
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
}
