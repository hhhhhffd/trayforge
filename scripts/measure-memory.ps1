# Measures the memory of the Trayforge process tree (trayforge.exe + WebView2 descendants) in three
# states: idle, window open, window closed. A second launch toggles the window through the
# single-instance plugin. Usage: powershell -File scripts\measure-memory.ps1 -Exe path\to\trayforge.exe
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [int]$SettleSeconds = 10,
    [string]$Identifier = 'io.github.hhhhhffd.trayforge'
)

# WebView2 children are matched by the app's user data folder (named after the bundle
# identifier) rather than by parent PID: Windows reuses PIDs, so a parent-chain walk can
# pick up unrelated processes.
function Get-Tree([int]$RootPid) {
    $webviews = Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -like "*$Identifier*" }
    @($RootPid) + @($webviews | ForEach-Object { [int]$_.ProcessId })
}

function Measure-Tree([string]$State, [int]$RootPid) {
    $ids = Get-Tree $RootPid
    $procs = Get-Process -Id @($ids) -ErrorAction SilentlyContinue
    # Private working set is not exposed by Get-Process; the counter set is.
    $private = 0
    foreach ($p in $procs) {
        $c = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess = $($p.Id)"
        if ($c) { $private += $c.WorkingSetPrivate }
    }
    [pscustomobject]@{
        State             = $State
        Processes         = @($procs).Count
        WebView2Children  = @($procs | Where-Object ProcessName -eq 'msedgewebview2').Count
        WorkingSetMB      = [math]::Round(($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 1)
        PrivateWorkingMB  = [math]::Round($private / 1MB, 1)
    }
}

$app = Start-Process -FilePath $Exe -PassThru
try {
    Start-Sleep -Seconds $SettleSeconds
    Measure-Tree 'idle' $app.Id
    Start-Process -FilePath $Exe -Wait          # second launch opens the window
    Start-Sleep -Seconds $SettleSeconds
    Measure-Tree 'window open' $app.Id
    Start-Process -FilePath $Exe -Wait          # second launch closes it again
    Start-Sleep -Seconds $SettleSeconds
    Measure-Tree 'closed' $app.Id
} finally {
    Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
}
