SELECT date
FROM attendance_day_status
WHERE class_id = ?1 AND status = 'closed'
ORDER BY date ASC;
