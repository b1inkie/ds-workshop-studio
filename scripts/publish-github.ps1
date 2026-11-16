# Publish a versioned GitHub Release with the portable ZIP and latest.json.
# Uses the token stored in the Git credential store; never prints it.
param([string]$Notes = '')
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$repo = 'b1inkie/ds-workshop-studio'

function Read-Text([string]$path) { [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8) }
function FileHash([string]$path) {
    $stream = [IO.File]::OpenRead($path); $sha = [Security.Cryptography.SHA256]::Create()
    try { return -join ($sha.ComputeHash($stream) | ForEach-Object { $_.ToString('x2') }) }
    finally { $sha.Dispose(); $stream.Dispose() }
}
function Token() {
    $store = Join-Path $HOME '.git-credentials'
    if (-not (Test-Path -LiteralPath $store)) { throw 'Git credential store is empty; authenticate with GitHub first.' }
    foreach ($line in [IO.File]::ReadAllLines($store)) {
        if ($line -match '^https://[^:]+:([^@]+)@github\.com/?$') { return $Matches[1] }
    }
    throw 'No github.com token found in the Git credential store.'
}

# 1. Version consistency across all manifests.
$package = (Read-Text (Join-Path $root 'package.json') | ConvertFrom-Json).version
$config = ((Read-Text (Join-Path $root 'src-tauri/tauri.conf.json')) | ConvertFrom-Json).version
$cargo = Select-String -Path (Join-Path $root 'src-tauri/Cargo.toml') -Pattern '^version = "(.+)"' | Select-Object -First 1
if ($package -ne $config -or $package -ne $cargo.Matches[0].Groups[1].Value) { throw "Version mismatch: package=$package tauri=$config cargo=$($cargo.Matches[0].Groups[1].Value)" }
$version = $package
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Invalid version: $version" }
$tag = "v$version"
$zipName = "Workshop-Studio-$version-Windows-test.zip"

# 2. Clean build of the portable release ZIP.
Push-Location $root
try {
    npm run desktop:build
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    npm run package:release
    if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed.' }
} finally { Pop-Location }
$zipPath = Join-Path $root ".local/releases/$zipName"
if (-not (Test-Path -LiteralPath $zipPath)) { throw "Missing release ZIP: $zipPath" }
$zipHash = FileHash $zipPath

# 3. Manifest consumed by the in-app updater (fetched via releases/latest/download/latest.json).
$manifest = [ordered]@{
    version = $version
    url = "https://github.com/$repo/releases/download/$tag/$zipName"
    sha256 = $zipHash
    notes = if ($Notes) { $Notes } else { "Workshop Studio $version" }
    pubDate = (Get-Date).ToString('yyyy-MM-dd')
}
$manifestPath = Join-Path $root '.local/releases/latest.json'
[IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json), (New-Object Text.UTF8Encoding($false)))

# 4. Create or reuse the release for this tag.
$token = Token
$headers = @{ Authorization = "Bearer $token"; Accept = 'application/vnd.github+json'; 'X-GitHub-Api-Version' = '2022-11-28' }
$release = $null
try { $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/tags/$tag" -Headers $headers } catch {}
if (-not $release) {
    $body = @{ tag_name = $tag; target_commitish = 'main'; name = "Workshop Studio $version"; body = $manifest.notes; draft = $false; prerelease = $false } | ConvertTo-Json
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases" -Method Post -Headers $headers -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($body))
    "Created release $tag"
} else {
    "Reusing existing release $tag"
}
# Retry-friendly: drop same-named assets before upload.
foreach ($asset in @($release.assets)) {
    if ($asset.name -in @($zipName, 'latest.json')) {
        Invoke-RestMethod -Uri $asset.url -Method Delete -Headers $headers | Out-Null
        "Deleted stale asset $($asset.name)"
    }
}
foreach ($upload in @(@($zipPath, $zipName, 'application/zip'), @($manifestPath, 'latest.json', 'application/octet-stream'))) {
    Invoke-RestMethod -Uri ("https://uploads.github.com/repos/$repo/releases/$($release.id)/assets?name=" + $upload[1]) -Method Post -Headers $headers -ContentType $upload[2] -InFile $upload[0] | Out-Null
    "Uploaded asset $($upload[1])"
}
Write-Host "Published $tag  ->  https://github.com/$repo/releases/tag/$tag"
Write-Host "Updater feed    ->  https://github.com/$repo/releases/latest/download/latest.json"
Write-Host "ZIP SHA256      ->  $zipHash"
