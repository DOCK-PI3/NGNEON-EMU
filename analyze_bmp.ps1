param([string]$path = "screenshots/auto_aof_neo.bmp")
if (!(Test-Path $path)) { Write-Host "FILE NOT FOUND: $path"; exit 1 }

$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $path).Path)
Write-Host ("File size: " + $bytes.Length + " bytes")

$bfType = [System.BitConverter]::ToUInt16($bytes, 0)
$bfSize = [System.BitConverter]::ToUInt32($bytes, 2)
$bfOffBits = [System.BitConverter]::ToUInt32($bytes, 10)
$biWidth = [System.BitConverter]::ToUInt32($bytes, 18)
$biHeight = [System.BitConverter]::ToUInt32($bytes, 22)
$biBitCount = [System.BitConverter]::ToUInt16($bytes, 28)

Write-Host ("Type: 0x{0:X4}" -f $bfType)
Write-Host ("Width: " + $biWidth + " Height: " + $biHeight + " BPP: " + $biBitCount)

$totalPixels = $biWidth * $biHeight
$nonBlack = 0
$sampleColors = @{}

for ($i = $bfOffBits; $i -lt $bytes.Length - 3; $i += 4) {
    $b = $bytes[$i]
    $g = $bytes[$i+1]
    $r = $bytes[$i+2]
    if ($r -gt 0 -or $g -gt 0 -or $b -gt 0) {
        $nonBlack++
        $key = "{0:X2}{1:X2}{2:X2}" -f $r, $g, $b
        if (-not $sampleColors.ContainsKey($key)) {
            $sampleColors[$key] = 1
            if ($sampleColors.Count -le 15) {
                Write-Host ("  Color: RGB({0},{1},{2}) hex=#{3}" -f $r,$g,$b,$key.ToLower())
            }
        }
    }
}

$pct = [math]::Round($nonBlack / $totalPixels * 100, 2)
Write-Host ("Non-black pixels: " + $nonBlack + " / " + $totalPixels + " (" + $pct + "%)")
Write-Host ("Unique non-black colors: " + $sampleColors.Count)

if ($nonBlack -gt 0) {
    Write-Host "VERDICT: GAME IS RENDERING! Framebuffer has content."
} else {
    Write-Host "VERDICT: All black - no game content rendered."
}
