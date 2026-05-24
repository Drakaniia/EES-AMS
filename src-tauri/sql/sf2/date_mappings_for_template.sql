SELECT template_id, sheet_name, date, column_letter, column_index
FROM sf2_date_mappings
WHERE template_id = ?1
ORDER BY date ASC;
