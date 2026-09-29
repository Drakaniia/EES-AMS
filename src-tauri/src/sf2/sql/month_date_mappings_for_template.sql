-- The day-number grid of one month, in date order.
--
-- Scoped by `template_id`, never by class. A month is a row, and replacing one
-- month's grid can never reach another's (spec section 4, factor 2).
--
-- `sheet_name` is selected too: one file holds twelve month worksheets, so the
-- column a day sits in is only addressable together with the sheet it sits on.
SELECT template_id, date, column_letter, column_index, sheet_name
FROM sf2_month_date_mappings
WHERE template_id = ?1
ORDER BY date ASC;
