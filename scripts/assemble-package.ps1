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

$mayaExe = Join-Path $repoRoot 'target/release/maya.exe'
if (-not (Test-Path -LiteralPath $mayaExe -PathType Leaf)) {
    throw "Release executable does not exist: $mayaExe"
}
Copy-Item -LiteralPath $mayaExe -Destination (Join-Path $releaseDir 'maya.exe')

$checksumFile = Join-Path $repoRoot 'FFmpeg/checksums.sha256'
foreach ($line in Get-Content -LiteralPath $checksumFile) {
    if ($line -notmatch '^([0-9A-Fa-f]{64})\s+(.+)$') {
        throw "Invalid FFmpeg checksum manifest line: $line"
    }
    $expected = $Matches[1].ToUpperInvariant()
    $name = $Matches[2]
    $source = Join-Path (Join-Path $repoRoot 'FFmpeg') $name
    $sourceHash = Get-Sha256 $source
    if ($sourceHash -ne $expected) {
        throw "Checksum mismatch before copy: $source"
    }
    $destination = Join-Path $releaseDir $name
    Copy-Item -LiteralPath $source -Destination $destination
    $destinationHash = Get-Sha256 $destination
    if ($destinationHash -ne $expected) {
        throw "Checksum mismatch after copy: $destination"
    }
}

Write-Host 'NPM release directory assembled with maya.exe, ffmpeg.exe, and ffprobe.exe.'
