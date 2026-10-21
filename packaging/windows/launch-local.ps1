param(
    [Parameter(Mandatory=$true)][string]$Configuration,
    [switch]$NoLaunch,
    [switch]$CheckOnly,
    [ValidatePattern('^spotify:[A-Za-z0-9:_%+./?=&,-]+$')][string]$OpenUri
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'local-source-update.psm1') -Force
$config = Get-Content -LiteralPath $Configuration -Raw | ConvertFrom-Json
$digest = [Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($config.Executable.ToLowerInvariant()))
$name = 'Local\SpotifastJSourceUpdate-' + ([BitConverter]::ToString($digest) -replace '-', '')
$mutex = New-Object Threading.Mutex($false, $name)
$owned = $false
try {
    try { $owned = $mutex.WaitOne(0) } catch [Threading.AbandonedMutexException] { $owned = $true }
    if (-not $owned) { return }
    $result = Invoke-LocalSourceUpdate -Configuration $Configuration -CheckOnly:$CheckOnly
    if ($NoLaunch -or $CheckOnly) { $result | ConvertTo-Json -Compress }
    else {
        $start = @{ FilePath=$config.Executable; WorkingDirectory=(Split-Path $config.Executable); WindowStyle='Normal' }
        if ($OpenUri) { $start.ArgumentList = $OpenUri }
        Start-Process @start | Out-Null
    }
} finally {
    if ($owned) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}
