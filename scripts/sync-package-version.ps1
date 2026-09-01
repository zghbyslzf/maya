$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$metadata = & cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $repoRoot 'Cargo.toml') | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed'
}
$cargoPackage = $metadata.packages | Where-Object { $_.name -eq 'maya' } | Select-Object -First 1
if ($null -eq $cargoPackage) {
    throw 'The maya package was not found in cargo metadata'
}

$packagePath = Join-Path $repoRoot 'pkg/package.json'
$json = Get-Content -Raw -LiteralPath $packagePath -Encoding UTF8
$versionPattern = '(?m)^(\s*"version"\s*:\s*)"([^"]+)"'
$versionMatch = [regex]::Match($json, $versionPattern)
if (-not $versionMatch.Success) {
    throw 'pkg/package.json does not contain a version field'
}
$oldVersion = $versionMatch.Groups[2].Value
$replacement = '$1"' + $cargoPackage.version + '"'
$json = [regex]::Replace($json, $versionPattern, $replacement, 1)
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($packagePath, $json.TrimEnd() + "`n", $utf8WithoutBom)

Write-Host "NPM version synchronized from $oldVersion to Cargo workspace version $($cargoPackage.version)."
