param(
    [Parameter(Mandatory = $true)]
    [string]$WorkbookPath,
    [Parameter(Mandatory = $true)]
    [string]$MarksPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'

function Set-Sf2Mark {
    param(
        $Sheet,
        [string]$CellAddress,
        [string]$Value
    )

    $cell = $Sheet.Range($CellAddress)
    $target = $cell
    if ($cell.MergeCells) {
        $target = $cell.MergeArea.Cells.Item(1, 1)
    }

    if ($target.HasFormula) {
        throw "Refusing to overwrite formula cell $($Sheet.Name)!$($target.Address($false, $false))"
    }

    if ($Value.Length -eq 0) {
        $target.Value2 = $null
    } else {
        $target.Value2 = $Value
    }
}

$marks = Get-Content -Path $MarksPath -Raw | ConvertFrom-Json
$excel = $null
$workbook = $null

try {
    $excel = New-Object -ComObject Excel.Application
    $excel.Visible = $false
    $excel.DisplayAlerts = $false
    $excel.EnableEvents = $false
    try { $excel.AutomationSecurity = 3 } catch {}

    $workbook = $excel.Workbooks.Open($WorkbookPath, 0, $false)

    foreach ($mark in $marks) {
        $sheet = $workbook.Worksheets.Item([string]$mark.sheetName)
        Set-Sf2Mark -Sheet $sheet -CellAddress ([string]$mark.cellAddress) -Value ([string]$mark.value)
    }

    $excel.CalculateFullRebuild()
    $workbook.Save()

    [ordered]@{
        outputPath = $OutputPath
        marksWritten = @($marks).Count
    } | ConvertTo-Json -Depth 3 -Compress
}
finally {
    if ($null -ne $workbook) { $workbook.Close($true) | Out-Null }
    if ($null -ne $excel) { $excel.Quit() | Out-Null }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
