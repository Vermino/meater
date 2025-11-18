# Download frontend libraries locally to avoid CORS/tracking prevention issues

$libs = @{
    "react.production.min.js" = "https://unpkg.com/react@18/umd/react.production.min.js"
    "react-dom.production.min.js" = "https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"
    "babel.min.js" = "https://unpkg.com/@babel/standalone/babel.min.js"
    "Recharts.js" = "https://unpkg.com/recharts@2.5.0/dist/Recharts.js"
    "tailwind.min.js" = "https://cdn.tailwindcss.com"
}

# Create libs directory
$libsDir = "libs"
if (!(Test-Path $libsDir)) {
    New-Item -ItemType Directory -Path $libsDir
}

Write-Host "Downloading frontend libraries..."

foreach ($lib in $libs.GetEnumerator()) {
    $outFile = Join-Path $libsDir $lib.Key
    Write-Host "Downloading $($lib.Key)..."
    try {
        Invoke-WebRequest -Uri $lib.Value -OutFile $outFile
        Write-Host "  ✓ Downloaded $($lib.Key) ($(([math]::Round((Get-Item $outFile).Length / 1KB, 2))) KB)"
    } catch {
        Write-Host "  ✗ Failed to download $($lib.Key): $_" -ForegroundColor Red
    }
}

Write-Host "`nDone! Libraries downloaded to $libsDir/"
Write-Host "Now update index.html to use local files instead of CDN."
