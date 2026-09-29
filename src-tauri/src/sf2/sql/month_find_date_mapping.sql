SELECT template_id, date, column_letter, column_index, sheet_name
FROM sf2_month_date_mappings
WHERE template_id = ?1
  AND date = ?2;
