# Download frontend libraries locally to avoid CORS/tracking prevention issues

$libs = @{
    "react.production.min.js" = "https://unpkg.com/react@18/umd/react.production.min.js"
    "react-dom.production.min.js" = "https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"
    "react-is.production.min.js" = "https://unpkg.com/react-is@18.2.0/umd/react-is.production.min.js"
    "babel.min.js" = "https://unpkg.com/@babel/standalone/babel.min.js"
    "Recharts.min.js" = "https://unpkg.com/recharts@2.4.3/umd/Recharts.min.js"
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
        $sizeKB = [math]::Round((Get-Item $outFile).Length / 1KB, 2)
        Write-Host "  Downloaded $($lib.Key) ($sizeKB KB)" -ForegroundColor Green
    } catch {
        Write-Host "  Failed to download $($lib.Key): $_" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "Done! Libraries downloaded to $libsDir/"
Write-Host "Now restart the web server and Tauri app."
