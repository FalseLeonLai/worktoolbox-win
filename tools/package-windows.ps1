param([switch]$Offline)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw '此脚本仅用于 Windows PowerShell 7。' }
$projectRoot = Split-Path $PSScriptRoot -Parent
$previousFlags = $env:RUSTFLAGS
Push-Location -LiteralPath $projectRoot
try {
    # MSVC 默认动态链接 VC 运行库；单文件交付使用静态 CRT。
    $env:RUSTFLAGS = ($previousFlags + ' -C target-feature=+crt-static').Trim()
    $buildArguments = @('build', '--release', '--locked', '--target', 'x86_64-pc-windows-msvc', '--target-dir', 'target/windows-portable')
    if ($Offline) { $buildArguments += '--offline' }
    & cargo @buildArguments
    if ($LASTEXITCODE -ne 0) { throw "构建失败，退出码 $LASTEXITCODE" }
    $versionLine = Select-String -LiteralPath 'Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    $version = $versionLine.Matches[0].Groups[1].Value
    $destination = Join-Path $projectRoot "target/delivery/工作工具箱-v$version.exe"
    New-Item -ItemType Directory -Path (Split-Path $destination -Parent) -Force | Out-Null
    Copy-Item -LiteralPath 'target/windows-portable/x86_64-pc-windows-msvc/release/worktoolbox-win.exe' -Destination $destination
    $artifact = Get-Item -LiteralPath $destination
    [pscustomobject]@{
        Path = $artifact.FullName
        Bytes = $artifact.Length
        SHA256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash
    } | Format-List
} finally {
    $env:RUSTFLAGS = $previousFlags
    Pop-Location
}
