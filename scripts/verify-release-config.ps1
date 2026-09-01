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
$metadata = & cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $repoRoot 'Cargo.toml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed'
}

$versions = @($metadata.packages | ForEach-Object { $_.version } | Sort-Object -Unique)
if ($versions.Count -ne 1) {
    throw "Workspace package versions differ: $($versions -join ', ')"
}

foreach ($package in $metadata.packages) {
    if ($null -eq $package.publish -or @($package.publish).Count -ne 0) {
        throw "Cargo package '$($package.name)' must set publish = false"
    }
}

$checksumFile = Join-Path $repoRoot 'FFmpeg/checksums.sha256'
foreach ($line in Get-Content -LiteralPath $checksumFile) {
    if ($line -notmatch '^([0-9A-Fa-f]{64})\s+(.+)$') {
        throw "Invalid FFmpeg checksum manifest line: $line"
    }
    $expected = $Matches[1].ToUpperInvariant()
    $binary = Join-Path (Join-Path $repoRoot 'FFmpeg') $Matches[2]
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Bundled FFmpeg tool does not exist: $binary"
    }
    $actual = Get-Sha256 $binary
    if ($actual -ne $expected) {
        throw "Bundled FFmpeg tool checksum mismatch: $binary"
    }
}

Write-Host "Release config valid: $($metadata.packages.Count) Cargo packages at $($versions[0]), publish disabled, FFmpeg checksums valid."
