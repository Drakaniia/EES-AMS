-- v21 — `sf2_month_date_mappings`: the day-number grid of one month file.
--
-- `sheet_name` is gone: a month file has exactly one worksheet, named
-- "{MONTH} {year}", so the sheet is derivable from
-- `sf2_month_templates.report_month` / `report_year`. That is what makes a
-- month switch a pure read of this table with no sheet resolution (spec §6.2).

CREATE TABLE IF NOT EXISTS sf2_month_date_mappings (
    template_id   TEXT NOT NULL,
    date          TEXT NOT NULL,
    column_letter TEXT NOT NULL,
    column_index  INTEGER NOT NULL,
    PRIMARY KEY(template_id, date),
    FOREIGN KEY(template_id) REFERENCES sf2_month_templates(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sf2_month_date_mappings_date
    ON sf2_month_date_mappings(date);
