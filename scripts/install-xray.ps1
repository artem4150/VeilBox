param()

$ErrorActionPreference = 'Stop'
$version = 'v26.9.9'
$sha256 = '244deaba2098c2964e49bba90df3707777e5f5f428a82d2f29604015f24beec2'
$destination = Join-Path (Split-Path -Parent $PSScriptRoot) 'src-tauri/bin'
$staging = Join-Path ([System.IO.Path]::GetTempPath()) ("veilbox-xray-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    $archive = Join-Path $staging 'xray.zip'
    Invoke-WebRequest -Uri "https://github.com/XTLS/Xray-core/releases/download/$version/Xray-windows-64.zip" -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sha256) {
        throw 'Xray archive SHA-256 mismatch; runtime files were not changed.'
    }
    $extracted = Join-Path $staging 'extracted'
    Expand-Archive -LiteralPath $archive -DestinationPath $extracted
    foreach ($name in @('xray.exe', 'geoip.dat', 'geosite.dat')) {
        Copy-Item -LiteralPath (Join-Path $extracted $name) -Destination (Join-Path $destination $name) -Force
    }
    Write-Host "Installed Xray $version with GeoIP/GeoSite data into $destination"
} finally {
    $absoluteStage = [System.IO.Path]::GetFullPath($staging)
    $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($absoluteStage.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase) -and
        (Split-Path -Leaf $absoluteStage).StartsWith('veilbox-xray-')) {
        Remove-Item -LiteralPath $absoluteStage -Recurse -Force
    }
}
