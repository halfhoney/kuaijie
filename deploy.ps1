# 快截一键发版：打包应用 → 上传安装包到 R2 → 部署官网到 Pages
# 用法: powershell -File deploy.ps1 [-Version 0.1.0] [-SkipBuild]
param(
    [string]$Version = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
# Cloudflare API 在国内需要 IPv4 优先，否则 wrangler 会超时
$env:NODE_OPTIONS = "--dns-result-order=ipv4first"

$ROOT = Split-Path -Parent $MyInvocation.MyCommand.Path
$VERSION = if ($Version) { $Version } else { (Get-Content "$ROOT\src-tauri\tauri.conf.json" | ConvertFrom-Json).version }
$EXE = "$ROOT\src-tauri\target\release\bundle\nsis\快截_${VERSION}_x64-setup.exe"
$R2_KEY = "kuaijie_${VERSION}_x64-setup.exe"
$R2_URL = "https://pub-203ffe576881479e9fb7a0aac6e1704e.r2.dev/$R2_KEY"

Write-Host "==> 版本: $VERSION" -ForegroundColor Green

# 1. 打包应用
if (-not $SkipBuild) {
    Write-Host "==> 打包应用 (tauri build)..." -ForegroundColor Green
    Push-Location $ROOT
    pnpm tauri build
    if ($LASTEXITCODE -ne 0) { throw "打包失败" }
    Pop-Location
}
if (-not (Test-Path $EXE)) { throw "安装包不存在: $EXE" }

# 2. 上传安装包到 R2
Write-Host "==> 上传安装包到 R2: $R2_KEY" -ForegroundColor Green
wrangler r2 object put "kuaijie-dl/$R2_KEY" --file="$EXE" --content-type="application/octet-stream" --remote
if ($LASTEXITCODE -ne 0) { throw "R2 上传失败" }

# 3. 更新官网里的版本号和下载链接
Write-Host "==> 更新官网版本号与下载链接" -ForegroundColor Green
$html = Get-Content "$ROOT\website\index.html" -Raw -Encoding UTF8
$html = $html -replace 'v\d+\.\d+\.\d+', "v$VERSION"
$html = $html -replace 'https://pub-203ffe576881479e9fb7a0aac6e1704e\.r2\.dev/[^"\s]+', $R2_URL
$html = $html -replace '(约 |~)43( MB| MB)', "`${1}43`${2}"  # 体积占位，如需精确请手动改
$html | Set-Content "$ROOT\website\index.html" -Encoding UTF8 -NoNewline

# 4. 部署官网
Write-Host "==> 部署官网到 Pages" -ForegroundColor Green
wrangler pages deploy "$ROOT\website" --project-name=kuaijie
if ($LASTEXITCODE -ne 0) { throw "Pages 部署失败" }

Write-Host ""
Write-Host "✅ 发版完成！" -ForegroundColor Green
Write-Host "   官网: https://kuaijie-61s.pages.dev"
Write-Host "   安装包: $R2_URL"
