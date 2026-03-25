param(
    [string]$InstallDir = "$env:USERPROFILE\.local\bin"
)

$ErrorActionPreference = "Stop"
$Repo = "ricky-ultimate/scriptvault"

Write-Host "Fetching latest ScriptVault release..."

$release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$version = $release.tag_name
$asset   = $release.assets | Where-Object { $_.name -eq "sv-windows-x86_64.zip" }

if (-not $asset) {
    Write-Error "Could not find Windows release asset for $version"
    exit 1
}

Write-Host "Installing ScriptVault $version..."

$tmp     = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_.FullName }
$archive = Join-Path $tmp "sv.zip"

Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archive
Expand-Archive -Path $archive -DestinationPath $tmp

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}

Move-Item -Force "$tmp\sv.exe" "$InstallDir\sv.exe"
Remove-Item -Recurse -Force $tmp

# Add to PATH for current user if not already there
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$InstallDir", "User")
    Write-Host ""
    Write-Host "Added $InstallDir to your PATH."
    Write-Host "Restart your terminal for the change to take effect."
}

Write-Host ""
Write-Host "✓ ScriptVault $version installed to $InstallDir\sv.exe"
Write-Host ""
& "$InstallDir\sv.exe" --version
