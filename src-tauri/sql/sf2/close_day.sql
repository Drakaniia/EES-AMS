INSERT INTO attendance_day_status (class_id, date, status, closed_at)
VALUES (?1, ?2, 'closed', ?3)
ON CONFLICT(class_id, date) DO UPDATE SET
    status = 'closed',
    closed_at = excluded.closed_at;
