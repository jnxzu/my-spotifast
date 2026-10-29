param([Parameter(Mandatory=$true)][string]$InstalledExecutable)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'local-source-update.psm1') -Force
$installed = Get-InstalledVersion $InstalledExecutable
$higher = [version]::new($installed.Major, $installed.Minor, $installed.Build + 1)
$prefix = Join-Path ([IO.Path]::GetTempPath()) 'spotifast-source-update-test-'
$root = $prefix + [Guid]::NewGuid().ToString('N')
$repo = Join-Path $root 'repo'
$target = Join-Path $root 'installed\spotifast.exe'
$candidate = Join-Path $repo 'target\local-install\release\spotifast.exe'
$configuration = Join-Path $root 'configuration.json'
$build = Join-Path $root 'build.ps1'
$marker = Join-Path $repo 'built.txt'
function Assert-Equal($Actual, $Expected, [string]$Why) {
    if ($Actual -ne $Expected) { throw "$Why - expected $Expected, got $Actual" }
}
function Write-Manifest([version]$Version) {
    "[package]`nname = `"fixture`"`nversion = `"$Version`"`n[dependencies]`nother = {version = `"999.0.0`"}" |
        Set-Content -LiteralPath (Join-Path $repo 'Cargo.toml') -Encoding UTF8
}
try {
    New-Item -ItemType Directory -Path (Split-Path $target),(Split-Path $candidate) -Force | Out-Null
    Copy-Item -LiteralPath $InstalledExecutable -Destination $target
    Copy-Item -LiteralPath $InstalledExecutable -Destination $candidate
    $hash = (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash
    @{RepositoryRoot=$repo;Executable=$target;PowerShell=(Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe');BuildScript=$build;RegistryKey=$null} |
        ConvertTo-Json | Set-Content -LiteralPath $configuration -Encoding UTF8
    'param($Configuration); [Console]::Error.WriteLine("Normal native build progress"); $cfg=Get-Content -LiteralPath $Configuration -Raw | ConvertFrom-Json; Set-Content -LiteralPath (Join-Path $cfg.RepositoryRoot "built.txt") -Value called; exit 17' |
        Set-Content -LiteralPath $build -Encoding UTF8
    foreach($version in @([version]'0.0.0', $installed)) {
        Write-Manifest $version
        Assert-Equal (Get-RepositoryVersion $repo) $version 'Only [package].version counts'
        $result = Invoke-LocalSourceUpdate -Configuration $configuration
        Assert-Equal $result.Result 'Skip' 'Lower or equal versions must skip'
        Assert-Equal (Test-Path -LiteralPath $marker) $false 'Skipping must never invoke the build'
        Assert-Equal (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash $hash 'Skipping must preserve the installed executable'
    }
    Write-Manifest $higher
    $check = Invoke-LocalSourceUpdate -Configuration $configuration -CheckOnly
    Assert-Equal $check.Result 'Build' 'A higher version needs a build'
    Assert-Equal (Test-Path -LiteralPath $marker) $false 'CheckOnly must not build'
    $failed = Invoke-LocalSourceUpdate -Configuration $configuration
    Assert-Equal $failed.Result 'KeptInstalled' 'Build failure must retain the installed app'
    Assert-Equal ($failed.Error -like 'Build failed; see *') $true 'Failure must come from the builder'
    Assert-Equal ((Get-Content -LiteralPath (Join-Path $root 'build.log') -Raw) -match 'Normal native build progress') $true 'Native stderr must be captured in the build log'
    Assert-Equal (Test-Path -LiteralPath $marker) $true 'A higher version must invoke the builder'
    Assert-Equal (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash $hash 'Failed builds must preserve the installed bytes'
    Assert-Equal (Test-Path -LiteralPath $candidate) $false 'Build artifacts must be removed after failure'
    Copy-Item -LiteralPath $InstalledExecutable -Destination $candidate
    (Get-Content -LiteralPath $build -Raw).Replace('exit 17','exit 0') | Set-Content -LiteralPath $build -Encoding UTF8
    $wrong = Invoke-LocalSourceUpdate -Configuration $configuration
    Assert-Equal $wrong.Result 'KeptInstalled' 'Wrong-version artifacts must be refused'
    Assert-Equal $wrong.Error 'Build output has the wrong version.' 'The stale artifact guard must reject the build'
    Assert-Equal ((Get-Content -LiteralPath (Join-Path $root 'build.log') -Raw) -match 'Normal native build progress') $true 'Successful build stderr must not abort output verification'
    Assert-Equal (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash $hash 'A stale artifact cannot replace the installed app'
    $racing = 'param($Configuration); $cfg=Get-Content -LiteralPath $Configuration -Raw | ConvertFrom-Json; Set-Content -LiteralPath (Join-Path $cfg.RepositoryRoot "Cargo.toml") -Value "[package]`nversion = `"999.0.0`""'
    Set-Content -LiteralPath $build -Value $racing -Encoding UTF8
    $changed = Invoke-LocalSourceUpdate -Configuration $configuration
    Assert-Equal $changed.Result 'KeptInstalled' 'A changing repository version must be refused'
    Assert-Equal ($changed.Error -like 'Repository version changed during the build*') $true 'The version race guard must reject the build'
    Assert-Equal (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash $hash 'A version race cannot replace the installed app'
    'version = "999.0.0"' | Set-Content -LiteralPath (Join-Path $repo 'Cargo.toml') -Encoding UTF8
    $invalid = Invoke-LocalSourceUpdate -Configuration $configuration
    Assert-Equal $invalid.Result 'KeptInstalled' 'Missing [package] must be refused'
    # Disposable programs provide real, distinct --version replies.
    $oldStub = Join-Path $root 'old-stub.exe'
    $newStub = Join-Path $root 'new-stub.exe'
    Add-Type -TypeDefinition 'public class OldStub { public static void Main() { System.Console.WriteLine("spotifast 0.0.1"); } }' -OutputAssembly $oldStub -OutputType ConsoleApplication
    Add-Type -TypeDefinition 'public class NewStub { public static void Main() { System.Console.WriteLine("spotifast 0.0.2"); } }' -OutputAssembly $newStub -OutputType ConsoleApplication
    $repoExe = Join-Path $repo 'spotifast.exe'
    Copy-Item -LiteralPath $oldStub -Destination $repoExe
    $readme = Join-Path $repo 'README.md'
    Set-Content -LiteralPath $readme -Value 'source readme must stay unchanged' -Encoding UTF8
    $readmeHash = (Get-FileHash -LiteralPath $readme -Algorithm SHA256).Hash
    @{RepositoryRoot=$repo;Executable=$repoExe;PowerShell=(Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe');BuildScript=$build;RegistryKey=$null} |
        ConvertTo-Json | Set-Content -LiteralPath $configuration -Encoding UTF8
    $builder = @(
        'param($Configuration)',
        '$cfg = Get-Content -Raw -LiteralPath $Configuration | ConvertFrom-Json',
        '$release = Join-Path $cfg.RepositoryRoot "target\local-install\release"',
        'New-Item -ItemType Directory -Path (Join-Path $release "deps") -Force | Out-Null',
        'Copy-Item -LiteralPath (Join-Path (Split-Path $cfg.RepositoryRoot) "new-stub.exe") -Destination (Join-Path $release "spotifast.exe")',
        'Copy-Item -LiteralPath (Join-Path (Split-Path $cfg.RepositoryRoot) "new-stub.exe") -Destination (Join-Path $release "deps\spotifast.exe")'
    ) -join "`r`n"
    Set-Content -LiteralPath $build -Value $builder -Encoding UTF8
    Write-Manifest ([version]'0.0.2')
    $updated = Invoke-LocalSourceUpdate -Configuration $configuration
    Assert-Equal $updated.Result 'Updated' 'A repo-root installation must update successfully'
    Assert-Equal (Get-InstalledVersion $repoExe) ([version]'0.0.2') 'The repository executable must be replaced'
    Assert-Equal (Get-FileHash -LiteralPath $readme -Algorithm SHA256).Hash $readmeHash 'The source README must not be copied onto itself'
    Assert-Equal (Test-Path -LiteralPath $candidate) $false 'The build executable must be removed'
    Assert-Equal (Test-Path -LiteralPath (Join-Path (Split-Path $candidate) 'deps\spotifast.exe')) $false 'The dependency build executable must be removed'
    Assert-Equal (Test-Path -LiteralPath (Join-Path $root 'previous\spotifast.rollback')) $false 'The temporary rollback copy must be removed after success'
    Write-Output 'Passed: version decisions, failures, stale output, version race, and repository-root update with rollback and cleanup.'
} finally {
    $resolved = [IO.Path]::GetFullPath($root)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath($prefix), [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to clean an unexpected fixture directory.'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
