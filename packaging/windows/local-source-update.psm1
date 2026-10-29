Set-StrictMode -Version Latest

function Get-RepositoryVersion([string]$RepositoryRoot) {
    $manifest = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'Cargo.toml') -Raw
    $package = [regex]::Match($manifest, '(?ms)^\[package\]\s*\r?\n(.*?)(?=^\[|\z)')
    $version = [regex]::Match($package.Groups[1].Value, '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*(?:#.*)?$')
    if (-not $package.Success -or -not $version.Success) { throw 'Cargo.toml needs a stable [package].version.' }
    return [version]$version.Groups[1].Value
}

function Get-InstalledVersion([string]$Executable) {
    $reported = (& $Executable --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $reported -notmatch '^spotifast (\d+\.\d+\.\d+)$') {
        throw 'Could not verify the executable version.'
    }
    return [version]$Matches[1]
}

function Get-SourceUpdateDecision([version]$RepositoryVersion, [version]$InstalledVersion) {
    if ($RepositoryVersion -gt $InstalledVersion) { return 'Build' }
    return 'Skip'
}

function Invoke-LocalSourceUpdate([string]$Configuration, [switch]$CheckOnly) {
    $config = Get-Content -LiteralPath $Configuration -Raw | ConvertFrom-Json
    $state = Split-Path -Parent $Configuration
    $executable = $config.Executable
    $result = [ordered]@{ Result = 'KeptInstalled'; InstalledVersion = $null; RepositoryVersion = $null; Error = $null }
    $log = Join-Path $state 'update.log'
    $staged = "$executable.local-update"
    $candidate = Join-Path $config.RepositoryRoot 'target\local-install\release\spotifast.exe'
    $candidateDeps = Join-Path $config.RepositoryRoot 'target\local-install\release\deps\spotifast.exe'
    $buildAttempted = $false
    function Write-UpdateLog([string]$Message) {
        Add-Content -LiteralPath $log -Value ("{0:o} {1}" -f (Get-Date), $Message) -Encoding UTF8
    }
    try {
        $installed = Get-InstalledVersion $executable
        $result.InstalledVersion = $installed.ToString()
        $repository = Get-RepositoryVersion $config.RepositoryRoot
        $result.RepositoryVersion = $repository.ToString()
        $decision = Get-SourceUpdateDecision $repository $installed
        if ($CheckOnly -or $decision -eq 'Skip') {
            $result.Result = $decision
            return [pscustomobject]$result
        }
        $running = @(Get-CimInstance Win32_Process -Filter "name='spotifast.exe'" |
            Where-Object { $_.ExecutablePath -eq $executable })
        if ($running.Count) {
            $result.Result = 'DeferredWhileRunning'
            return [pscustomobject]$result
        }
        Write-UpdateLog "Building local $repository; installed $installed."
        $buildLog = Join-Path $state 'build.log'
        # Native stderr includes normal Cargo progress. Drain both pipes without
        # turning those messages into terminating PowerShell 5.1 errors.
        if ($config.BuildScript.Contains('"') -or $Configuration.Contains('"')) {
            throw 'Build configuration paths cannot contain quotes.'
        }
        $start = New-Object Diagnostics.ProcessStartInfo
        $start.FileName = $config.PowerShell
        $start.Arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "{0}" -Configuration "{1}"' -f $config.BuildScript, $Configuration
        $start.UseShellExecute = $false
        $start.CreateNoWindow = $true
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        $process = New-Object Diagnostics.Process
        $process.StartInfo = $start
        try {
            $buildAttempted = $true
            if (-not $process.Start()) { throw 'Could not start the local build.' }
            $stdout = $process.StandardOutput.ReadToEndAsync()
            $stderr = $process.StandardError.ReadToEndAsync()
            $process.WaitForExit()
            $stdout.GetAwaiter().GetResult() + $stderr.GetAwaiter().GetResult() |
                Set-Content -LiteralPath $buildLog -Encoding UTF8
            if ($process.ExitCode -ne 0) { throw "Build failed; see $buildLog" }
        } finally {
            $process.Dispose()
        }
        if ((Get-RepositoryVersion $config.RepositoryRoot) -ne $repository) {
            throw 'Repository version changed during the build; keeping the installed copy.'
        }
        if ((Get-InstalledVersion $candidate) -ne $repository) { throw 'Build output has the wrong version.' }
        $hash = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash
        $running = @(Get-CimInstance Win32_Process -Filter "name='spotifast.exe'" |
            Where-Object { $_.ExecutablePath -eq $executable })
        if ($running.Count) {
            $result.Result = 'DeferredWhileRunning'
            return [pscustomobject]$result
        }
        $backupRoot = Join-Path $state 'previous'
        New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null
        $backup = Join-Path $backupRoot 'spotifast.rollback'
        $readme = Join-Path (Split-Path $executable) 'README.md'
        $oldReadme = Join-Path $backupRoot 'README.md'
        $newReadme = Join-Path $config.RepositoryRoot 'README.md'
        $copyReadme = -not [string]::Equals([IO.Path]::GetFullPath($readme), [IO.Path]::GetFullPath($newReadme), [StringComparison]::OrdinalIgnoreCase)
        $hadReadme = $copyReadme -and (Test-Path -LiteralPath $readme)
        if ($hadReadme) { Copy-Item -LiteralPath $readme -Destination $oldReadme -Force }
        $metadata = $null
        if ($config.RegistryKey) {
            $metadata = Get-ItemProperty -LiteralPath $config.RegistryKey
            if ($metadata.InstallLocation.TrimEnd('\') -ne (Split-Path -Parent $executable)) {
                throw 'The uninstall record belongs to another installation.'
            }
        }
        Copy-Item -LiteralPath $candidate -Destination $staged -Force
        if ((Get-FileHash -LiteralPath $staged -Algorithm SHA256).Hash -ne $hash) {
            throw 'Staged executable checksum mismatch.'
        }
        [IO.File]::Replace($staged, $executable, $backup)
        try {
            if ((Get-InstalledVersion $executable) -ne $repository -or
                (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash -ne $hash) {
                throw 'Installed executable verification failed.'
            }
            if ($metadata) {
                Set-ItemProperty -LiteralPath $config.RegistryKey -Name DisplayVersion -Value $repository.ToString()
                Set-ItemProperty -LiteralPath $config.RegistryKey -Name DisplayName -Value "Spotifast J $repository"
            }
            if ($copyReadme -and (Test-Path -LiteralPath $newReadme)) { Copy-Item -LiteralPath $newReadme -Destination $readme -Force }
        } catch {
            Copy-Item -LiteralPath $backup -Destination $staged -Force
            $failed = Join-Path $backupRoot 'failed.rollback'
            [IO.File]::Replace($staged, $executable, $failed)
            Remove-Item -LiteralPath $failed, $backup -Force -ErrorAction SilentlyContinue
            if ($hadReadme) { Copy-Item -LiteralPath $oldReadme -Destination $readme -Force }
            if ($metadata) {
                Set-ItemProperty -LiteralPath $config.RegistryKey -Name DisplayVersion -Value $metadata.DisplayVersion
                Set-ItemProperty -LiteralPath $config.RegistryKey -Name DisplayName -Value $metadata.DisplayName
            }
            throw
        }
        $result.Result = 'Updated'
        $result.InstalledVersion = $repository.ToString()
        try { Remove-Item -LiteralPath $backup -Force -ErrorAction Stop }
        catch { Write-UpdateLog "Could not remove temporary rollback copy: $($_.Exception.Message)" }
        Write-UpdateLog "Installed and verified $repository ($hash)."
    } catch {
        $result.Error = $_.Exception.Message
        Write-UpdateLog $result.Error
    } finally {
        if (Test-Path -LiteralPath $staged) { Remove-Item -LiteralPath $staged -Force }
        if ($buildAttempted) {
            foreach ($artifact in @($candidate, $candidateDeps)) {
                if (Test-Path -LiteralPath $artifact) {
                    try { Remove-Item -LiteralPath $artifact -Force -ErrorAction Stop }
                    catch { Write-UpdateLog "Could not remove build executable $artifact`: $($_.Exception.Message)" }
                }
            }
        }
    }
    return [pscustomobject]$result
}

Export-ModuleMember -Function Get-RepositoryVersion, Get-InstalledVersion, Get-SourceUpdateDecision, Invoke-LocalSourceUpdate
