ALTER TABLE students ADD COLUMN gender TEXT CHECK(gender IS NULL OR gender IN ('male', 'female'));

UPDATE students
SET gender = (
    SELECT lower(mapping.gender_block)
    FROM sf2_student_mappings AS mapping
    WHERE
        mapping.student_id = students.id
        AND mapping.gender_block IN ('MALE', 'FEMALE')
    ORDER BY mapping.template_id
    LIMIT 1
)
WHERE
    gender IS NULL
    AND EXISTS (
        SELECT 1
        FROM sf2_student_mappings AS mapping
        WHERE
            mapping.student_id = students.id
            AND mapping.gender_block IN ('MALE', 'FEMALE')
    );
