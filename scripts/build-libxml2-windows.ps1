# Build static libxml2 for MSVC and install libxml2s.lib next to the LLVM prefix.
# LLVM Windows packages list libxml2s.lib in `llvm-config --system-libs` but do not ship it.
param(
    [string]$InstallDir = "C:\llvm\lib",
    [string]$Version = "2.12.7"
)

$ErrorActionPreference = "Stop"
$dest = Join-Path $InstallDir "libxml2s.lib"
if (Test-Path $dest) {
    Write-Host "libxml2s.lib already present at $dest"
    exit 0
}

New-Item -Force -ItemType Directory -Path $InstallDir | Out-Null
$work = Join-Path $env:RUNNER_TEMP "libxml2-build"
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -Force -ItemType Directory -Path $work | Out-Null

$tar = Join-Path $work "libxml2.tar.xz"
$srcDir = Join-Path $work "libxml2-$Version"
$url = "https://download.gnome.org/sources/libxml2/2.12/libxml2-$Version.tar.xz"
Write-Host "Downloading libxml2 $Version..."
curl.exe -fSL --retry 3 --connect-timeout 30 -o $tar $url
tar -xf $tar -C $work

$buildDir = Join-Path $work "build"
cmake -S $srcDir -B $buildDir -G "Visual Studio 17 2022" -A x64 `
    -DBUILD_SHARED_LIBS=OFF `
    -DLIBXML2_WITH_PYTHON=OFF `
    -DLIBXML2_WITH_ZLIB=OFF `
    -DLIBXML2_WITH_LZMA=OFF `
    -DLIBXML2_WITH_ICU=OFF `
    -DLIBXML2_WITH_MODULES=OFF `
    -DLIBXML2_WITH_PROGRAMS=OFF `
    -DLIBXML2_WITH_TESTS=OFF `
    -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL `
    -DCMAKE_C_FLAGS="/DLIBXML_STATIC"

cmake --build $buildDir --config Release --parallel
$built = Join-Path $buildDir "Release\libxml2s.lib"
if (-not (Test-Path $built)) {
    throw "Expected static library missing: $built"
}
Copy-Item $built $dest -Force
Write-Host "Installed $dest"
