# install.ps1 — Install sshm-rs from the latest GitHub release on Windows.
# Usage: irm https://raw.githubusercontent.com/bit5hift/sshm-rs/master/install.ps1 | iex

$ErrorActionPreference = "Stop"

$repo = "bit5hift/sshm-rs"
$target = "x86_64-pc-windows-msvc"
$apiUrl = "https://api.github.com/repos/$repo/releases/latest"

function Info($msg)  { Write-Host "[info]  $msg" -ForegroundColor Blue }
function Ok($msg)    { Write-Host "[ok]    $msg" -ForegroundColor Green }
function Err($msg)   { Write-Host "[error] $msg" -ForegroundColor Red; exit 1 }

# Determine install directory
$installDir = Join-Path $env:USERPROFILE ".local\bin"

# Fetch latest release
Info "Fetching latest release from GitHub..."
try {
    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "sshm-rs-installer" }
} catch {
    Err "Failed to fetch release info: $_"
}

$version = $release.tag_name
if (-not $version) { Err "Could not determine latest version." }
Info "Latest release: $version"

$archiveName = "sshm-rs-$version-$target.zip"
$downloadUrl = "https://github.com/$repo/releases/download/$version/$archiveName"

# Download
$tmpDir = Join-Path $env:TEMP "sshm-rs-install"
if (Test-Path $tmpDir) { Remove-Item -Recurse -Force $tmpDir }
New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

$archivePath = Join-Path $tmpDir $archiveName
Info "Downloading $archiveName..."
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath -UseBasicParsing
} catch {
    Err "Download failed: $_"
}

# Extract
Info "Extracting to $installDir..."
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force

# Copy binaries
$extractedFiles = Get-ChildItem -Path $tmpDir -Recurse -Include "sshm-rs.exe"
foreach ($file in $extractedFiles) {
    Copy-Item -Path $file.FullName -Destination $installDir -Force
}

# Cleanup
Remove-Item -Recurse -Force $tmpDir

# Check PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    Info "$installDir is not in your PATH. Adding it..."
    $newPath = "$installDir;$userPath"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$installDir;$env:Path"
    Ok "Added $installDir to user PATH (restart your terminal to apply)."
} else {
    Ok "$installDir is already in PATH."
}

Ok "sshm-rs $version installed to $installDir"
Ok "Run 'sshm-rs' to get started."
