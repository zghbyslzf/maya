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
$artifactRecord = Join-Path $repoRoot 'target/maya-release-artifact.json'
$artifactRecordDirectory = Split-Path -Parent $artifactRecord
if (-not (Test-Path -LiteralPath $artifactRecordDirectory)) {
    New-Item -ItemType Directory -Path $artifactRecordDirectory | Out-Null
}
if (Test-Path -LiteralPath $artifactRecord) {
    Remove-Item -LiteralPath $artifactRecord -Force
}

$cargoArguments = @(
    'build',
    '--release',
    '--locked',
    '--package',
    'maya',
    '--message-format=json-render-diagnostics'
)
$messages = & cargo @cargoArguments
if ($LASTEXITCODE -ne 0) {
    throw 'Cargo release build failed'
}

$mayaExe = $null
foreach ($line in $messages) {
    try {
        $message = $line | ConvertFrom-Json -ErrorAction Stop
    }
    catch {
        continue
    }
    if ($message.reason -eq 'compiler-artifact' -and $message.target.name -eq 'maya' -and $message.target.kind -contains 'bin' -and $message.executable) {
        $mayaExe = [System.IO.Path]::GetFullPath([string]$message.executable)
    }
}
if (-not $mayaExe -or -not (Test-Path -LiteralPath $mayaExe -PathType Leaf)) {
    throw 'Cargo did not report the maya release executable path'
}

$artifact = [ordered]@{
    executable = $mayaExe
    sha256 = Get-Sha256 $mayaExe
}
$json = $artifact | ConvertTo-Json
$utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($artifactRecord, $json.TrimEnd() + "`n", $utf8WithoutBom)

Write-Host "Cargo release artifact recorded: $mayaExe"
