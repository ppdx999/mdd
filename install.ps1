$ErrorActionPreference = "Stop"

$repo = "ppdx999/mdd"
$installDir = if ($env:MDD_INSTALL_DIR) { $env:MDD_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }
$target = "x86_64-pc-windows-msvc"

# Get latest version if not specified
if (-not $env:MDD_VERSION) {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $version = $release.tag_name
} else {
    $version = $env:MDD_VERSION
}

if (-not $version) {
    Write-Error "Failed to determine latest version"
    exit 1
}

$archive = "mdd-$version-$target.zip"
$url = "https://github.com/$repo/releases/download/$version/$archive"

Write-Host "Installing mdd $version for $target..."
Write-Host "  from: $url"
Write-Host "  to:   $installDir"

# Download and extract
$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    $archivePath = Join-Path $tmpDir $archive
    Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing
    Expand-Archive -Path $archivePath -DestinationPath $tmpDir

    # Install binaries
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    $extractedDir = Join-Path $tmpDir "mdd-$version-$target"
    Get-ChildItem "$extractedDir\mdd*.exe" | ForEach-Object {
        Copy-Item $_.FullName -Destination $installDir
        Write-Host "  installed: $($_.Name)"
    }
} finally {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Done! Make sure $installDir is in your PATH."
Write-Host ""
Write-Host "  To add to PATH (current user):"
Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `"$installDir;`$env:PATH`", 'User')"
Write-Host ""
