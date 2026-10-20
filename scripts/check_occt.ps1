<#
.SYNOPSIS
  Locate an OpenCASCADE (OCCT) installation and validate it for `cad_kernel`.

.DESCRIPTION
  Prints the resolved OCCT_ROOT, checks for headers and libraries, and shows
  the environment/commands needed to build with the OCCT backend.

.EXAMPLE
  pwsh -File scripts/check_occt.ps1
  $env:OCCT_ROOT = "C:\vcpkg\installed\x64-windows"; pwsh -File scripts/check_occt.ps1
#>

[CmdletBinding()]
param(
    [string]$Root = $env:OCCT_ROOT
)

function Find-Occt {
    param([string]$Explicit)

    $candidates = @()
    if ($Explicit) { $candidates += $Explicit }
    if ($env:VCPKG_INSTALLATION_ROOT) {
        $candidates += (Join-Path $env:VCPKG_INSTALLATION_ROOT "installed\x64-windows")
    }
    $candidates += @(Get-ChildItem "C:\" -Directory -Filter "OpenCASCADE*" -ErrorAction SilentlyContinue |
        ForEach-Object { $_.FullName })
    $candidates += @(Get-ChildItem "C:\" -Directory -Filter "vcpkg*" -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName "installed\x64-windows" })

    foreach ($c in $candidates) {
        if ($c -and (Test-Path -LiteralPath $c)) { return $c }
    }
    return $null
}

function Test-Includes {
    param([string]$Path)
    $a = Join-Path $Path "include\opencascade\Standard_Version.hxx"
    $b = Join-Path $Path "include\Standard_Version.hxx"
    foreach ($p in @($a, $b)) {
        if (Test-Path -LiteralPath $p) { return $p }
    }
    return $null
}

function Test-Libs {
    param([string]$Path)
    $dirs = @(
        (Join-Path $Path "lib"),
        (Join-Path $Path "win64\vc14\lib"),
        (Join-Path $Path "win64\vc14\bin")
    )
    foreach ($d in $dirs) {
        if (Test-Path -LiteralPath $d) {
            $hit = Get-ChildItem -LiteralPath $d -Filter "TKernel.*" -ErrorAction SilentlyContinue |
                Select-Object -First 1
            if ($hit) { return $hit.FullName }
        }
    }
    return $null
}

Write-Host "PolyForge - OCCT prerequisite check" -ForegroundColor Cyan

if (-not $Root) {
    Write-Host "[--] OCCT_ROOT is not set." -ForegroundColor Yellow
    Write-Host "     vcpkg:  vcpkg install opencascade:x64-windows"
    Write-Host "     then:   `$env:OCCT_ROOT = `"<vcpkg>\installed\x64-windows`""
    exit 1
}

if (-not (Test-Path -LiteralPath $Root)) {
    Write-Host "[!!] OCCT_ROOT does not exist: $Root" -ForegroundColor Red
    exit 1
}

Write-Host "[ok] OCCT_ROOT = $Root"

$header = Test-Includes -Path $Root
if ($header) {
    $version = $null
    $m = Select-String -LiteralPath $header -Pattern '#define\s+OCC_VERSION_COMPLETE\s+"([^"]+)"' -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($m) { $version = $m.Matches.Groups[1].Value }
    Write-Host "[ok] headers: $header" -ForegroundColor Green
    if ($version) { Write-Host "[ok] version: $version" -ForegroundColor Green }
} else {
    Write-Host "[!!] Standard_Version.hxx not found (expected include\opencascade or include)" -ForegroundColor Red
    exit 1
}

$lib = Test-Libs -Path $Root
if ($lib) {
    Write-Host "[ok] libraries: $lib" -ForegroundColor Green
} else {
    Write-Host "[!!] TKernel not found in lib / win64\vc14\lib / win64\vc14\bin" -ForegroundColor Red
    exit 1
}

$bin = Join-Path $Root "bin"
Write-Host ""
Write-Host "Build:" -ForegroundColor Cyan
Write-Host "  `$env:OCCT_ROOT = `"$Root`""
if (Test-Path -LiteralPath $bin) { Write-Host "  `$env:PATH = `"$bin;`$env:PATH`"   # runtime DLLs" }
Write-Host "  cargo test -p polyforge-cad --features cad_kernel"
exit 0
