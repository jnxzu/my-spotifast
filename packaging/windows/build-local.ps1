param([Parameter(Mandatory=$true)][string]$Configuration)
$ErrorActionPreference = 'Stop'
$config = Get-Content -LiteralPath $Configuration -Raw | ConvertFrom-Json
foreach($entry in $config.BuildEnvironment.PSObject.Properties) {
    if ($entry.Name -notin @('CARGO_HOME','RUSTUP_HOME','VCPKG_INSTALLATION_ROOT','LIBCLANG_PATH')) {
        throw 'Unexpected build environment key.'
    }
    Set-Item -LiteralPath "Env:$($entry.Name)" -Value $entry.Value
}
$cargo = Join-Path $env:CARGO_HOME 'bin\cargo.exe'
$env:PATH = (Split-Path $cargo) + ';' + $env:PATH
$env:VCPKG_DISABLE_METRICS = '1'
# A new toolchain requirement must fail safely rather than download at launch.
# https://rust-lang.github.io/rustup/environment-variables.html
$env:RUSTUP_AUTO_INSTALL = '0'
if (-not $env:VSCMD_VER) {
    Import-Module (Join-Path $config.VisualStudioRoot 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
    Enter-VsDevShell -VsInstallPath $config.VisualStudioRoot -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
}
Set-Location -LiteralPath $config.RepositoryRoot
$start = New-Object Diagnostics.ProcessStartInfo
$start.FileName = $cargo
$start.Arguments = 'build --release --locked --offline --target-dir target/local-install'
$start.WorkingDirectory = $config.RepositoryRoot
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$process = New-Object Diagnostics.Process
$process.StartInfo = $start
try {
    if (-not $process.Start()) { throw 'Could not start Cargo.' }
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    [Console]::Out.Write($stdout.GetAwaiter().GetResult())
    [Console]::Error.Write($stderr.GetAwaiter().GetResult())
    if ($process.ExitCode -ne 0) { throw "Cargo build failed with exit code $($process.ExitCode)." }
} finally {
    $process.Dispose()
}
