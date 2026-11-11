$ErrorActionPreference = 'Stop'
$workspace = Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$localTools = Join-Path $workspace 'work\tools'
$nodeRuntime = Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin'
if (Test-Path $nodeRuntime) { $env:PATH = "$nodeRuntime;$env:PATH" }
if (Test-Path (Join-Path $localTools 'npm.cmd')) { $env:PATH = "$localTools;$env:PATH" }
if (Test-Path (Join-Path $localTools 'cargo\bin\cargo.exe')) {
  $env:CARGO_HOME = Join-Path $localTools 'cargo'
  $env:RUSTUP_HOME = Join-Path $localTools 'rustup'
  $env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
}
Set-Location $PSScriptRoot
if (!(Test-Path node_modules)) {
  & npm.cmd install
  if ($LASTEXITCODE -ne 0) { throw 'Dependency installation failed.' }
}
& npm.cmd run tauri dev

