"""Render SF2 sheets to PNG via Excel COM, and dump geometry (widths/heights).

Usage:
  python tools/sf2-diff/render.py pdf <file> <sheet> <out.pdf>
  python tools/sf2-diff/render.py png <file> <sheet> <out.png> [zoom]
  python tools/sf2-diff/render.py geom <file> <sheet> [maxCol] [maxRow]
"""

from __future__ import annotations

import sys
from pathlib import Path


def col_letter(idx: int) -> str:
    s = ""
    idx += 1
    while idx:
        idx, rem = divmod(idx - 1, 26)
        s = chr(65 + rem) + s
    return s


def export_pdf(src: Path, sheet: str, out_pdf: Path) -> None:
    import win32com.client

    excel = win32com.client.Dispatch("Excel.Application")
    excel.Visible = False
    excel.DisplayAlerts = False
    try:
        wb = excel.Workbooks.Open(str(src), ReadOnly=True)
        ws = wb.Worksheets(sheet)
        ws.PageSetup.Orientation = 2  # landscape
        ws.PageSetup.Zoom = False
        ws.PageSetup.FitToPagesWide = 1
        ws.PageSetup.FitToPagesTall = False
        ws.ExportAsFixedFormat(0, str(out_pdf.resolve()))  # 0 = xlTypePDF
        wb.Close(SaveChanges=False)
    finally:
        excel.Quit()


def pdf_to_png(pdf: Path, out_png: Path, zoom: float) -> None:
    import fitz

    doc = fitz.open(str(pdf))
    page = doc[0]
    pix = page.get_pixmap(matrix=fitz.Matrix(zoom, zoom))
    pix.save(str(out_png))


def dump_geometry(src: Path, sheet: str, max_col: int, max_row: int) -> None:
    import win32com.client

    excel = win32com.client.Dispatch("Excel.Application")
    excel.Visible = False
    excel.DisplayAlerts = False
    try:
        wb = excel.Workbooks.Open(str(src), ReadOnly=True)
        ws = wb.Worksheets(sheet)
        used = ws.UsedRange
        last_col = min(used.Columns.Count + used.Column - 1, max_col)
        last_row = min(used.Rows.Count + used.Row - 1, max_row)
        print(f"SHEET {sheet}  usedRange={used.Address}")
        widths = []
        for c in range(1, last_col + 1):
            w = ws.Columns(c).ColumnWidth
            widths.append(f"{col_letter(c - 1)}={w}")
        print("WIDTHS:", " | ".join(widths))
        heights = []
        for r in range(1, last_row + 1):
            h = ws.Rows(r).RowHeight
            heights.append(f"{r}={h}")
        print("HEIGHTS:", " | ".join(heights))
        wb.Close(SaveChanges=False)
    finally:
        excel.Quit()


def main() -> None:
    mode = sys.argv[1]
    if mode == "pdf":
        export_pdf(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    elif mode == "png":
        pdf = Path(sys.argv[4] + ".pdf")
        export_pdf(Path(sys.argv[2]), sys.argv[3], pdf)
        zoom = float(sys.argv[5]) if len(sys.argv) > 5 else 2.0
        pdf_to_png(pdf, Path(sys.argv[4]), zoom)
    elif mode == "geom":
        max_col = int(sys.argv[4]) if len(sys.argv) > 4 else 50
        max_row = int(sys.argv[5]) if len(sys.argv) > 5 else 90
        dump_geometry(Path(sys.argv[2]), sys.argv[3], max_col, max_row)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()