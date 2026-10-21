param(
    [Parameter(Mandatory=$true)][string]$Configuration,
    [string]$StateRoot = (Join-Path $env:LOCALAPPDATA 'Spotifast J Source Update')
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'local-source-update.psm1') -Force
$config = Get-Content -LiteralPath $Configuration -Raw | ConvertFrom-Json
$null = Get-RepositoryVersion $config.RepositoryRoot
$null = Get-InstalledVersion $config.Executable
$running = @(Get-CimInstance Win32_Process -Filter "name='spotifast.exe'" | Where-Object { $_.ExecutablePath -eq $config.Executable })
if ($running.Count) { throw 'Close the installed app before configuring its local updater.' }
New-Item -ItemType Directory -Path $StateRoot -Force | Out-Null
$savedConfig = Join-Path $StateRoot 'configuration.json'
$launcher = Join-Path $StateRoot 'launch.vbs'
$template = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'launch-local.vbs') -Raw
$template = $template.Replace('{POWERSHELL}', $config.PowerShell)
$template = $template.Replace('{SCRIPT}', (Join-Path $config.RepositoryRoot 'packaging\windows\launch-local.ps1'))
$template = $template.Replace('{CONFIGURATION}', $savedConfig).Replace('{EXECUTABLE}', $config.Executable)
$config | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $savedConfig -Encoding UTF8
[IO.File]::WriteAllText($launcher, $template, [Text.Encoding]::Unicode)
if ($config.SettingsFile -and (Test-Path -LiteralPath $config.SettingsFile)) {
    $settings = [IO.File]::ReadAllText($config.SettingsFile)
    $pattern = '("check_for_updates"\s*:\s*)true\b'
    if ([regex]::Matches($settings, $pattern).Count -eq 1) {
        $updated = [regex]::Replace($settings, $pattern, '${1}false')
        $stagedSettings = $config.SettingsFile + '.source-updater'
        [IO.File]::WriteAllText($stagedSettings, $updated, (New-Object Text.UTF8Encoding($false)))
        [IO.File]::Replace($stagedSettings, $config.SettingsFile, (Join-Path $StateRoot 'previous-settings.json'))
    }
}
$hostExe = Join-Path $env:WINDIR 'System32\wscript.exe'
$shell = New-Object -ComObject WScript.Shell
$shortcuts = @()
foreach($root in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop'),
    (Join-Path $env:APPDATA 'Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar'))) {
    foreach($file in @(Get-ChildItem -LiteralPath $root -Filter '*Spotifast*.lnk' -Recurse -ErrorAction SilentlyContinue)) {
        $link = $shell.CreateShortcut($file.FullName)
        if ($link.TargetPath -eq $config.Executable -and -not $link.Arguments) {
            $backup = Join-Path $StateRoot ("shortcut-{0}.lnk" -f $shortcuts.Count)
            if (-not (Test-Path -LiteralPath $backup)) { Copy-Item -LiteralPath $file.FullName -Destination $backup }
            $link.TargetPath = $hostExe
            $link.Arguments = '//B //Nologo "' + $launcher + '"'
            $link.IconLocation = $config.Executable + ',0'
            $link.WorkingDirectory = Split-Path $config.Executable
            $link.Save()
            $shortcuts += @{ Path=$file.FullName; Backup=$backup }
        }
    }
}
if ($shortcuts.Count) { $shortcuts | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $StateRoot 'shortcut-backups.json') -Encoding UTF8 }
$protocols = @()
foreach($key in @('HKCU:\Software\Classes\spotify\shell\open\command', 'HKCU:\Software\Classes\SpotifastJ.spotify\shell\open\command')) {
    if (Test-Path -LiteralPath $key) {
        $previous = (Get-Item -LiteralPath $key).GetValue('')
        if ($previous -and $previous.Contains($config.Executable)) {
            $protocols += @{ Key=$key; Command=$previous }
            Set-Item -LiteralPath $key -Value ('"' + $hostExe + '" //B //Nologo "' + $launcher + '" "%1"')
        }
    }
}
if ($protocols.Count) { $protocols | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $StateRoot 'protocol-backups.json') -Encoding UTF8 }
[pscustomobject]@{Configuration=$savedConfig;Launcher=$launcher;Shortcuts=$shortcuts.Count;Protocols=$protocols.Count} | ConvertTo-Json
