pub(super) fn class_name(grade_level: &str, section: &str) -> String {
    let grade = grade_level.trim();
    let section = section.trim();
    match (grade.is_empty(), section.is_empty()) {
        (false, false) => format!("{grade} - {section}"),
        (false, true) => grade.to_string(),
        (true, false) => section.to_string(),
        (true, true) => "SF2 Class".to_string(),
    }
}

pub(super) fn sanitize_file_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
