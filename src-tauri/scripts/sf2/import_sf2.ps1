param(
    [Parameter(Mandatory = $true)]
    [string]$Path
)

$ErrorActionPreference = 'Stop'

function Convert-ColumnNumberToLetter {
    param([int]$Column)
    $letter = ''
    while ($Column -gt 0) {
        $modulo = ($Column - 1) % 26
        $letter = [char](65 + $modulo) + $letter
        $Column = [math]::Floor(($Column - $modulo) / 26)
    }
    return $letter
}

function Get-MonthNumber {
    param([string]$Name)
    $normalized = $Name.ToUpperInvariant()
    if ($normalized -match 'JAN') { return 1 }
    if ($normalized -match 'FEB') { return 2 }
    if ($normalized -match 'MAR') { return 3 }
    if ($normalized -match 'APR') { return 4 }
    if ($normalized -match 'MAY') { return 5 }
    if ($normalized -match 'JUN') { return 6 }
    if ($normalized -match 'JUL') { return 7 }
    if ($normalized -match 'AUG') { return 8 }
    if ($normalized -match 'SEP') { return 9 }
    if ($normalized -match 'OCT') { return 10 }
    if ($normalized -match 'NOV') { return 11 }
    if ($normalized -match 'DEC') { return 12 }
    return 0
}

function Get-YearFromSheetName {
    param([string]$Name)
    if ($Name -match '(20\d{2})') {
        return [int]$Matches[1]
    }
    return 0
}

$excel = $null
$workbook = $null

try {
    $excel = New-Object -ComObject Excel.Application
    $excel.Visible = $false
    $excel.DisplayAlerts = $false
    $excel.EnableEvents = $false
    try { $excel.AutomationSecurity = 3 } catch {}

    $workbook = $excel.Workbooks.Open($Path, 0, $true)
    $sheets = @()
    $dates = @()
    $learners = @()
    $schoolYear = ''
    $schoolId = ''
    $schoolName = ''
    $reportMonth = ''
    $gradeLevel = ''
    $section = ''
    $adviserName = ''
    $schoolHeadName = ''
    $firstMonthlySheet = $null

    foreach ($sheet in $workbook.Worksheets) {
        $usedRange = $sheet.UsedRange
        $sheetInfo = [ordered]@{
            name = [string]$sheet.Name
            visible = [int]$sheet.Visible
            usedRange = [string]$usedRange.Address($false, $false)
        }
        $sheets += $sheetInfo

        if ([int]$sheet.Visible -ne -1) {
            continue
        }

        $monthNumber = Get-MonthNumber $sheet.Name
        $year = Get-YearFromSheetName $sheet.Name
        if ($monthNumber -eq 0 -or $year -eq 0) {
            continue
        }

        if ($null -eq $firstMonthlySheet) {
            $firstMonthlySheet = $sheet
            $schoolId = [string]$sheet.Cells.Item(3, 6).Text
            $schoolName = [string]$sheet.Cells.Item(4, 6).Text
            $schoolYear = [string]$sheet.Cells.Item(3, 13).Text
            $reportMonth = [string]$sheet.Cells.Item(3, 27).Text
            $gradeLevel = [string]$sheet.Cells.Item(4, 27).Text
            $section = [string]$sheet.Cells.Item(4, 39).Text
            $adviserName = [string]$sheet.Cells.Item(76, 40).Text
            if ($adviserName.Trim().Length -eq 0) {
                $adviserName = [string]$sheet.Cells.Item(82, 26).Text
            }
            $schoolHeadName = [string]$sheet.Cells.Item(82, 40).Text
        }

        for ($column = 6; $column -le 38; $column++) {
            $dayText = ([string]$sheet.Cells.Item(6, $column).Text).Trim()
            $day = 0
            if ([int]::TryParse($dayText, [ref]$day) -and $day -ge 1 -and $day -le 31) {
                $date = Get-Date -Year $year -Month $monthNumber -Day $day -Format 'yyyy-MM-dd'
                $dates += [ordered]@{
                    sheetName = [string]$sheet.Name
                    date = [string]$date
                    columnLetter = Convert-ColumnNumberToLetter $column
                    columnIndex = $column
                }
            }
        }
    }

    if ($null -ne $firstMonthlySheet) {
        $usedRange = $firstMonthlySheet.UsedRange
        $rowCount = [int]$usedRange.Rows.Count
        $genderBlock = 'MALE'

        for ($row = 1; $row -le $rowCount; $row++) {
            $name = ([string]$firstMonthlySheet.Cells.Item($row, 3).Text).Trim()
            if ($name.Length -eq 0) {
                continue
            }

            $upperName = $name.ToUpperInvariant()
            if ($upperName -match 'MALE' -and $upperName -match 'TOTAL') {
                $genderBlock = 'FEMALE'
                continue
            }
            if ($upperName -match 'FEMALE' -and $upperName -match 'TOTAL') {
                $genderBlock = $null
                continue
            }

            $learners += [ordered]@{
                rowIndex = $row
                name = $name
                genderBlock = $genderBlock
            }
        }
    }

    $result = [ordered]@{
        fileFormat = [int]$workbook.FileFormat
        hasVbProject = [bool]$workbook.HasVBProject
        schoolId = $schoolId.Trim()
        schoolName = $schoolName.Trim()
        schoolYear = $schoolYear.Trim()
        reportMonth = $reportMonth.Trim()
        gradeLevel = $gradeLevel.Trim()
        section = $section.Trim()
        adviserName = $adviserName.Trim()
        schoolHeadName = $schoolHeadName.Trim()
        learners = $learners
        dates = $dates
        sheets = $sheets
    }

    $result | ConvertTo-Json -Depth 8 -Compress
}
finally {
    if ($null -ne $workbook) { $workbook.Close($false) | Out-Null }
    if ($null -ne $excel) { $excel.Quit() | Out-Null }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
