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
$packageDir = Join-Path $repoRoot 'pkg'
$releaseDir = Join-Path $packageDir 'release'
$mayaExe = Join-Path $releaseDir 'maya.exe'
$ffmpegExe = Join-Path $releaseDir 'ffmpeg.exe'
$ffprobeExe = Join-Path $releaseDir 'ffprobe.exe'

foreach ($path in @($mayaExe, $ffmpegExe, $ffprobeExe)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Release file does not exist: $path"
    }
}
$releaseFiles = @(Get-ChildItem -LiteralPath $releaseDir -File | ForEach-Object { $_.Name } | Sort-Object)
$expectedReleaseFiles = @('ffmpeg.exe', 'ffprobe.exe', 'maya.exe')
if (($releaseFiles -join ',') -ne ($expectedReleaseFiles -join ',')) {
    throw "Release directory contains unexpected files: $($releaseFiles -join ', ')"
}

$help = & $mayaExe --help | Out-String
if ($LASTEXITCODE -ne 0) {
    throw 'maya.exe --help failed'
}

$metadata = & cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $repoRoot 'Cargo.toml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed'
}
$cargoVersion = ($metadata.packages | Where-Object { $_.name -eq 'maya' } | Select-Object -First 1).version
$binaryVersion = (& $mayaExe --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $binaryVersion -ne "maya $cargoVersion") {
    throw "Maya executable version mismatch: expected maya $cargoVersion, got '$binaryVersion'"
}
foreach ($command in @('clean', 'git', 'pack', 'optimize', 'transform')) {
    if ($help -notmatch "(?m)^\s*$command\s") {
        throw "maya.exe --help is missing subcommand: $command"
    }
}
foreach ($option in @('--quiet', '--no-progress')) {
    if (-not $help.Contains($option)) {
        throw "maya.exe --help is missing global option: $option"
    }
}

$optimizeHelp = & $mayaExe optimize --help | Out-String
$packHelp = & $mayaExe pack --help | Out-String
$transformHelp = & $mayaExe transform --help | Out-String
foreach ($option in @('--new-file', '--jpeg-quality', '--failure-policy')) {
    if (-not $optimizeHelp.Contains($option)) {
        throw "optimize --help is missing option: $option"
    }
}
if (-not $packHelp.Contains('--out-dir')) {
    throw 'pack --help is missing --out-dir'
}
if (-not $transformHelp.Contains('--failure-policy')) {
    throw 'transform --help is missing --failure-policy'
}

& $ffmpegExe -version *> $null
if ($LASTEXITCODE -ne 0) {
    throw 'Bundled FFmpeg is not executable'
}
& $ffprobeExe -version *> $null
if ($LASTEXITCODE -ne 0) {
    throw 'Bundled FFprobe is not executable'
}

$checksumFile = Join-Path $repoRoot 'FFmpeg/checksums.sha256'
foreach ($line in Get-Content -LiteralPath $checksumFile) {
    if ($line -notmatch '^([0-9A-Fa-f]{64})\s+(.+)$') {
        throw "Invalid FFmpeg checksum manifest line: $line"
    }
    $actual = Get-Sha256 (Join-Path $releaseDir $Matches[2])
    if ($actual -ne $Matches[1].ToUpperInvariant()) {
        throw "Release FFmpeg checksum mismatch: $($Matches[2])"
    }
}

$readme = Get-Content -Raw -LiteralPath (Join-Path $packageDir 'README.md') -Encoding UTF8
if ($readme -match '(?m)^\s*maya\s+-(c|g|p|o|t)\b') {
    throw 'pkg/README.md still contains legacy single-letter flag examples'
}

$previousLocation = Get-Location
try {
    Set-Location -LiteralPath $packageDir
    $packOutput = & npm.cmd pack --dry-run --json --ignore-scripts 2>$null | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw 'npm pack --dry-run failed'
    }
}
finally {
    Set-Location -LiteralPath $previousLocation
}
$packResult = $packOutput | ConvertFrom-Json
$paths = @($packResult[0].files | ForEach-Object { $_.path -replace '\\', '/' })
foreach ($expected in @('release/maya.exe', 'release/ffmpeg.exe', 'release/ffprobe.exe', 'README.md', 'package.json')) {
    if ($paths -notcontains $expected) {
        throw "NPM package is missing file: $expected"
    }
}
if ($paths.Count -ne 5) {
    throw "NPM package contains unexpected files: $($paths -join ', ')"
}

$npmVersion = (Get-Content -Raw -LiteralPath (Join-Path $packageDir 'package.json') | ConvertFrom-Json).version
if ($cargoVersion -ne $npmVersion) {
    throw "Cargo and NPM versions differ: $cargoVersion / $npmVersion"
}

Write-Host "Package smoke test passed: CLI help, FFmpeg checksums, NPM contents, and version $cargoVersion are valid."
