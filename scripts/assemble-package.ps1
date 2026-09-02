$ErrorActionPreference = 'Stop'

function Get-Sha256([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '')
    }
    finally {
        $algorithm.Dispose()
        $stream.Dispose()
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$releaseDir = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'pkg/release'))
$expectedReleaseDir = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'pkg/release'))
if ($releaseDir -ne $expectedReleaseDir -or -not $releaseDir.StartsWith([System.IO.Path]::GetFullPath((Join-Path $repoRoot 'pkg')))) {
    throw "Refusing to clean unexpected directory: $releaseDir"
}

if (Test-Path -LiteralPath $releaseDir) {
    Remove-Item -LiteralPath $releaseDir -Recurse -Force
}
New-Item -ItemType Directory -Path $releaseDir | Out-Null

$artifactRecord = Join-Path $repoRoot 'target/maya-release-artifact.json'
if (-not (Test-Path -LiteralPath $artifactRecord -PathType Leaf)) {
    throw "Cargo release artifact record does not exist: $artifactRecord"
}
$artifact = Get-Content -Raw -LiteralPath $artifactRecord -Encoding UTF8 | ConvertFrom-Json
$mayaExe = [System.IO.Path]::GetFullPath([string]$artifact.executable)
if (-not (Test-Path -LiteralPath $mayaExe -PathType Leaf)) {
    throw "Recorded release executable does not exist: $mayaExe"
}
$mayaHash = Get-Sha256 $mayaExe
if ($mayaHash -ne ([string]$artifact.sha256).ToUpperInvariant()) {
    throw "Recorded release executable changed after build: $mayaExe"
}
$mayaDestination = Join-Path $releaseDir 'maya.exe'
Copy-Item -LiteralPath $mayaExe -Destination $mayaDestination
if ((Get-Sha256 $mayaDestination) -ne $mayaHash) {
    throw "Maya executable checksum mismatch after copy: $mayaDestination"
}

$metadata = & cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $repoRoot 'Cargo.toml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed'
}
$cargoVersion = ($metadata.packages | Where-Object { $_.name -eq 'maya' } | Select-Object -First 1).version
$binaryVersion = (& $mayaDestination --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $binaryVersion -ne "maya $cargoVersion") {
    throw "Maya executable version mismatch: expected maya $cargoVersion, got '$binaryVersion'"
}

$checksumFile = Join-Path $repoRoot 'FFmpeg/checksums.sha256'
foreach ($line in Get-Content -LiteralPath $checksumFile) {
    if ($line -notmatch '^([0-9A-Fa-f]{64})\s+(.+)$') {
        throw "Invalid FFmpeg checksum manifest line: $line"
    }
    $expected = $Matches[1].ToUpperInvariant()
    $name = $Matches[2]
    $source = Join-Path (Split-Path -Parent $mayaExe) $name
    $sourceHash = Get-Sha256 $source
    if ($sourceHash -ne $expected) {
        throw "Release sidecar checksum mismatch before package copy: $source"
    }
    $destination = Join-Path $releaseDir $name
    Copy-Item -LiteralPath $source -Destination $destination
    $destinationHash = Get-Sha256 $destination
    if ($destinationHash -ne $expected) {
        throw "Checksum mismatch after copy: $destination"
    }
}

Write-Host 'NPM release directory assembled with maya.exe, ffmpeg.exe, and ffprobe.exe.'
