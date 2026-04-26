# P2P Dual Node Test Script (PowerShell)
# Runs godot in background jobs (inherits environment variables)

$PROJECT_ROOT = Split-Path -Parent $PSScriptRoot
$CLIENT_DIR = "$PROJECT_ROOT\client"

Write-Host "=== Agentora P2P Dual Node Test ===" -ForegroundColor Cyan
Write-Host "Project Root: $PROJECT_ROOT"
Write-Host ""

# Check godot command
$godotPath = Get-Command godot -ErrorAction SilentlyContinue
if (-not $godotPath) {
    Write-Host "ERROR: godot not found in PATH" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "Starting Node A (Seed Node, Port 4001)..." -ForegroundColor Green
$env:AGENTORA_SIM_CONFIG = "../config/sim_node_a.toml"
# Start-Job runs in background and inherits environment
$jobA = Start-Job -ScriptBlock {
    $env:AGENTORA_SIM_CONFIG = "../config/sim_node_a.toml"
    godot --path $using:CLIENT_DIR
}
Write-Host "Node A job started: $($jobA.Id)"

Write-Host "Waiting 5 seconds..." -ForegroundColor Gray
Start-Sleep -Seconds 5

Write-Host ""
Write-Host "Starting Node B (Client Node, Port 4002)..." -ForegroundColor Green
$env:AGENTORA_SIM_CONFIG = "../config/sim_node_b.toml"
$jobB = Start-Job -ScriptBlock {
    $env:AGENTORA_SIM_CONFIG = "../config/sim_node_b.toml"
    godot --path $using:CLIENT_DIR
}
Write-Host "Node B job started: $($jobB.Id)"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Dual nodes started as background jobs!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Jobs running. Check taskbar for Godot windows." -ForegroundColor Yellow
Write-Host ""
Write-Host "To check job output:"
Write-Host "  Receive-Job -Id $($jobA.Id)"
Write-Host ""
Write-Host "To stop jobs:"
Write-Host "  Stop-Job -Id $($jobA.Id), $($jobB.Id)"
Write-Host ""
Read-Host "Press Enter to keep jobs running and exit"