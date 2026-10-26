# Builds the Windows setup program (decision 0169) from a release build:
# target/release/<exe>, LICENSE, NOTICE and licenses/ go into dist/, and
# Inno Setup makes dist/<APP_ID>-setup.exe from installer/windows/setup.iss.
# Every name comes from crates/app/src/branding.rs and crates/app/Cargo.toml,
# so a rename needs nothing here (RENAMING.md).
#
# Used by the CI Build job (to check it on every pull request) and by the
# Release workflow. Run from the repository's root, after
# `cargo build --release -p app`. Inno Setup is installed if missing.

$ErrorActionPreference = "Stop"

function Constant($text, $name) {
    $match = [regex]::Match($text, "pub const ${name}: &str\s*=\s*`"([^`"]+)`"")
    if (-not $match.Success) { throw "$name not found in branding.rs" }
    return $match.Groups[1].Value
}

$branding = Get-Content "crates/app/src/branding.rs" -Raw
$appName = Constant $branding "APP_NAME"
$appId = Constant $branding "APP_ID"
$extension = Constant $branding "PROJECT_EXTENSION"
$releases = Constant $branding "RELEASES_URL"

$appToml = Get-Content "crates/app/Cargo.toml" -Raw
$exe = [regex]::Match($appToml, '\[\[bin\]\]\s*name\s*=\s*"([^"]+)"').Groups[1].Value + ".exe"
$rootToml = Get-Content "Cargo.toml" -Raw
$version = [regex]::Match($rootToml, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"').Groups[1].Value
if ($env:APP_BUILD) { $version = "$version.$($env:APP_BUILD)" }

# The program and its licences, side by side.
New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item "target/release/$exe", LICENSE, NOTICE dist/
Copy-Item -Recurse -Force licenses dist/licenses

$iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $iscc)) {
    choco install innosetup --no-progress -y | Out-Null
}
if (-not (Test-Path $iscc)) { throw "Inno Setup could not be installed" }

$dist = (Resolve-Path dist).Path
$icon = (Resolve-Path assets/icon.ico).Path
& $iscc /Qp `
    "/DAppName=$appName" "/DAppId=$appId" "/DAppVersion=$version" `
    "/DExeName=$exe" "/DExtension=$extension" "/DReleasesUrl=$releases" `
    "/DSourceDir=$dist" "/DOutputDir=$dist" "/DIconFile=$icon" `
    installer/windows/setup.iss
if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed ($LASTEXITCODE)" }

"setup=$dist\$appId-setup.exe" | Out-File -Append -Encoding utf8 $env:GITHUB_OUTPUT
"name=$appName" | Out-File -Append -Encoding utf8 $env:GITHUB_OUTPUT
