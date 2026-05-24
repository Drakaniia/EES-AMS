param(
    [Parameter(Mandatory = $true)]
    [string]$WorkbookPath,
    [Parameter(Mandatory = $true)]
    [string]$MetadataPath
)

$ErrorActionPreference = 'Stop'

function Get-MetadataValue {
    param([string]$Name)
    $value = $metadata.$Name
    if ($null -eq $value) {
        return ''
    }
    return [string]$value
}

function Get-MetadataBool {
    param([string]$Name)
    $value = $metadata.$Name
    if ($null -eq $value) {
        return $false
    }
    return [bool]$value
}

function Get-MetadataInt {
    param(
        [string]$Name,
        [int]$Default
    )
    $value = $metadata.$Name
    if ($null -eq $value) {
        return $Default
    }

    $parsed = 0
    if ([int]::TryParse([string]$value, [ref]$parsed)) {
        return $parsed
    }

    return $Default
}

function Set-Sf2Cell {
    param(
        $Sheet,
        [int]$Row,
        [int]$Column,
        [string]$Value
    )

    $cell = $Sheet.Cells.Item($Row, $Column)
    $target = $cell
    if ($cell.MergeCells) {
        $target = $cell.MergeArea.Cells.Item(1, 1)
    }

    if ($target.HasFormula) {
        throw "Refusing to overwrite formula cell $($Sheet.Name)!$($target.Address($false, $false))"
    }

    try { $target.NumberFormat = '@' } catch {}
    if ($Value.Length -eq 0) {
        $target.Value2 = $null
    } else {
        $target.Value2 = $Value
    }
}

function Set-Sf2DateCell {
    param(
        $Sheet,
        [int]$Column,
        [string]$Value
    )

    Set-Sf2Cell $Sheet 6 $Column $Value

    $cell = $Sheet.Cells.Item(6, $Column)
    $target = $cell
    if ($cell.MergeCells) {
        $target = $cell.MergeArea.Cells.Item(1, 1)
        try { $cell.MergeArea.HorizontalAlignment = -4131 } catch {}
        try { $cell.MergeArea.IndentLevel = 0 } catch {}
    }

    try { $target.HorizontalAlignment = -4131 } catch {}
    try { $target.IndentLevel = 0 } catch {}
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

function Get-MonthName {
    param([int]$Month)
    $names = @(
        '',
        'JANUARY',
        'FEBRUARY',
        'MARCH',
        'APRIL',
        'MAY',
        'JUNE',
        'JULY',
        'AUGUST',
        'SEPTEMBER',
        'OCTOBER',
        'NOVEMBER',
        'DECEMBER'
    )
    return $names[$Month]
}

function Get-ReportYear {
    param(
        [string]$SchoolYear,
        [int]$Month
    )

    if ($SchoolYear -match '(20\d{2})\D+(20\d{2})') {
        if ($Month -ge 6) {
            return [int]$Matches[1]
        }
        return [int]$Matches[2]
    }

    return [int](Get-Date).Year
}

function Get-WeekdayIndex {
    param([string]$Label)
    switch ($Label.ToUpperInvariant()) {
        'M' { return 0 }
        'T' { return 1 }
        'W' { return 2 }
        'TH' { return 3 }
        'F' { return 4 }
        default { return -1 }
    }
}

function Get-DateWeekdayIndex {
    param([datetime]$Date)
    switch ($Date.DayOfWeek) {
        'Monday' { return 0 }
        'Tuesday' { return 1 }
        'Wednesday' { return 2 }
        'Thursday' { return 3 }
        'Friday' { return 4 }
        default { return -1 }
    }
}

function Get-WeekdayLabel {
    param([int]$Index)
    switch ($Index) {
        0 { return 'M' }
        1 { return 'T' }
        2 { return 'W' }
        3 { return 'TH' }
        4 { return 'F' }
        default { return '' }
    }
}

function Get-Sf2WeekdaySlots {
    param($Sheet)

    $slots = @()
    $weekIndex = 0
    $previousWeekday = -1

    for ($column = 6; $column -le 38; $column++) {
        $weekdayText = ([string]$Sheet.Cells.Item(7, $column).Text).Trim()
        $weekdayIndex = Get-WeekdayIndex $weekdayText
        if ($weekdayIndex -lt 0) {
            continue
        }

        if ($previousWeekday -ge 0 -and $weekdayIndex -le $previousWeekday) {
            $weekIndex += 1
        }

        $slots += [pscustomobject]@{
            Column = $column
            WeekIndex = $weekIndex
            WeekdayIndex = $weekdayIndex
            Label = Get-WeekdayLabel $weekdayIndex
        }
        $previousWeekday = $weekdayIndex
    }

    return $slots
}

function Get-FirstSchoolDay {
    param(
        [int]$Year,
        [int]$Month
    )

    $date = Get-Date -Year $Year -Month $Month -Day 1
    while ((Get-DateWeekdayIndex $date) -lt 0) {
        $date = $date.AddDays(1)
    }

    return $date.Date
}

function Set-Sf2MonthDates {
    param(
        $Sheet,
        [int]$Year,
        [int]$Month,
        [int]$FirstSchoolDay
    )

    $slots = @(Get-Sf2WeekdaySlots $Sheet)
    if ($slots.Count -eq 0) {
        return
    }

    $lastDay = [datetime]::DaysInMonth($Year, $Month)
    if ($FirstSchoolDay -lt 1 -or $FirstSchoolDay -gt $lastDay) {
        throw "First attendance day must be between 1 and $lastDay for this report month"
    }

    $firstSchoolDate = Get-Date -Year $Year -Month $Month -Day $FirstSchoolDay
    if ((Get-DateWeekdayIndex $firstSchoolDate) -lt 0) {
        throw "First attendance day must be a Monday-Friday school day"
    }

    $mondayAnchor = $firstSchoolDate.Date.AddDays(-1 * (Get-DateWeekdayIndex $firstSchoolDate))
    $daysBySlot = @{}

    for ($day = 1; $day -le $lastDay; $day++) {
        $date = Get-Date -Year $Year -Month $Month -Day $day
        if ($date.Date -lt $firstSchoolDate.Date) {
            continue
        }

        $weekdayIndex = Get-DateWeekdayIndex $date
        if ($weekdayIndex -lt 0) {
            continue
        }

        $weekIndex = [int][math]::Floor(($date.Date - $mondayAnchor).TotalDays / 7)
        $daysBySlot["$weekIndex-$weekdayIndex"] = [string]$day
    }

    foreach ($slot in $slots) {
        $key = "$($slot.WeekIndex)-$($slot.WeekdayIndex)"
        $value = ''
        if ($daysBySlot.ContainsKey($key)) {
            $value = [string]$daysBySlot[$key]
        }
        Set-Sf2DateCell $Sheet $slot.Column $value
        Set-Sf2Cell $Sheet 7 $slot.Column $slot.Label
    }
}

function Clear-Sf2MonthDates {
    param($Sheet)

    foreach ($slot in @(Get-Sf2WeekdaySlots $Sheet)) {
        Set-Sf2Cell $Sheet 6 $slot.Column ''
    }
}

function Rename-SheetUnique {
    param(
        $Sheet,
        [string]$BaseName
    )

    $name = $BaseName
    if ($name.Length -gt 31) {
        $name = $name.Substring(0, 31)
    }

    try {
        $Sheet.Name = $name
        return
    } catch {
        $suffix = 1
        while ($suffix -le 99) {
            $candidate = $name
            $tail = "-$suffix"
            if (($candidate.Length + $tail.Length) -gt 31) {
                $candidate = $candidate.Substring(0, 31 - $tail.Length)
            }
            $candidate = "$candidate$tail"
            try {
                $Sheet.Name = $candidate
                return
            } catch {
                $suffix += 1
            }
        }
        throw
    }
}

$metadata = Get-Content -Path $MetadataPath -Raw | ConvertFrom-Json
$excel = $null
$workbook = $null
$sheetsUpdated = 0

try {
    $excel = New-Object -ComObject Excel.Application
    $excel.Visible = $false
    $excel.DisplayAlerts = $false
    $excel.EnableEvents = $false
    try { $excel.AutomationSecurity = 3 } catch {}

    $workbook = $excel.Workbooks.Open($WorkbookPath, 0, $false)

    $sf2Sheets = @()
    $monthlySheets = @()

    foreach ($sheet in $workbook.Worksheets) {
        $title = ([string]$sheet.Cells.Item(1, 1).Text).Trim()
        if ($title -notmatch 'School Form 2') {
            continue
        }

        $sf2Sheets += $sheet

        if ((Get-MonthNumber $sheet.Name) -gt 0 -and ([string]$sheet.Name) -match '(20\d{2})') {
            $monthlySheets += $sheet
        }

        Set-Sf2Cell $sheet 3 6 (Get-MetadataValue 'schoolId')
        Set-Sf2Cell $sheet 3 13 (Get-MetadataValue 'schoolYear')
        Set-Sf2Cell $sheet 3 27 (Get-MetadataValue 'reportMonth')
        Set-Sf2Cell $sheet 4 6 (Get-MetadataValue 'schoolName')
        Set-Sf2Cell $sheet 4 27 (Get-MetadataValue 'gradeLevel')
        Set-Sf2Cell $sheet 4 39 (Get-MetadataValue 'section')

        $adviserName = Get-MetadataValue 'adviserName'
        Set-Sf2Cell $sheet 76 40 $adviserName
        Set-Sf2Cell $sheet 82 26 $adviserName
        Set-Sf2Cell $sheet 82 40 (Get-MetadataValue 'schoolHeadName')

        $sheetsUpdated += 1
    }

    if ((Get-MetadataBool 'configureCalendar') -and $monthlySheets.Count -gt 0) {
        $monthNumber = Get-MonthNumber (Get-MetadataValue 'reportMonth')
        if ($monthNumber -eq 0) {
            throw "Report Month must be a valid month name"
        }

        $reportYear = Get-ReportYear (Get-MetadataValue 'schoolYear') $monthNumber
        $targetSheetName = "$(Get-MonthName $monthNumber) $reportYear"
        $targetSheet = $null

        foreach ($sheet in $monthlySheets) {
            if ([string]$sheet.Name -eq $targetSheetName) {
                $targetSheet = $sheet
                break
            }
        }

        if ($null -eq $targetSheet) {
            $targetSheet = $monthlySheets[0]
        }

        $targetSheet.Visible = -1
        Rename-SheetUnique $targetSheet $targetSheetName
        $firstSchoolDay = Get-MetadataInt 'firstSchoolDay' 1
        Set-Sf2MonthDates $targetSheet $reportYear $monthNumber $firstSchoolDay
        try { $targetSheet.Activate() | Out-Null } catch {}

        $hiddenIndex = 1
        foreach ($sheet in $sf2Sheets) {
            if ([int]$sheet.Index -eq [int]$targetSheet.Index) {
                continue
            }

            Clear-Sf2MonthDates $sheet
            if ((Get-MonthNumber $sheet.Name) -gt 0 -and ([string]$sheet.Name) -match '(20\d{2})') {
                Rename-SheetUnique $sheet "__SF2_HIDDEN_$hiddenIndex"
            }
            $sheet.Visible = 0
            $hiddenIndex += 1
        }
    }

    $excel.CalculateFullRebuild()
    $workbook.Save()

    [ordered]@{
        workbookPath = $WorkbookPath
        sheetsUpdated = $sheetsUpdated
    } | ConvertTo-Json -Depth 3 -Compress
}
finally {
    if ($null -ne $workbook) { $workbook.Close($true) | Out-Null }
    if ($null -ne $excel) { $excel.Quit() | Out-Null }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
