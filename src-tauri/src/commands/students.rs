use super::*;
use crate::sf2::repository::Sf2Repository;

const CREATE_STUDENT_REQUIRES_SF2_WORKBOOK: &str =
    "Create an SF2 workbook for this class before adding students.";
const CREATE_STUDENT_REQUIRES_SF2_CLASS: &str =
    "Students must be assigned to an SF2 workbook class before they can be created.";

// ── Student Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_students(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Vec<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.list_by_class(class_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.get(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_student_by_card(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    serial: String,
) -> std::result::Result<Option<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.find_by_card(&serial).map_err(|e| e.to_string())
}

fn sync_sf2_roster_after_student_change(
    pool: &Pool<SqliteConnectionManager>,
    class_id: Option<&str>,
) {
    let Some(class_id) = class_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };

    if let Err(error) = service::sync_workbook_roster_for_class(pool.clone(), class_id) {
        log::warn!("failed to sync SF2 roster after student change for class {class_id}: {error}");
    }
}

fn ensure_sf2_workbook_exists_for_student_create(
    pool: &Pool<SqliteConnectionManager>,
    class_id: Option<&str>,
) -> std::result::Result<(), String> {
    let class_id = class_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CREATE_STUDENT_REQUIRES_SF2_CLASS.to_string())?;

    let sf2_repo = Sf2Repository::new(pool.clone());
    if sf2_repo
        .latest_template_for_class(class_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err(CREATE_STUDENT_REQUIRES_SF2_WORKBOOK.to_string());
    }

    Ok(())
}

fn student_roster_fields_changed(before: &Student, after: &Student) -> bool {
    before.name != after.name || before.gender != after.gender || before.class_id != after.class_id
}

#[tauri::command]
pub fn create_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateStudentRequest,
) -> std::result::Result<Student, String> {
    ensure_sf2_workbook_exists_for_student_create(pool.inner(), req.class_id.as_deref())?;
    let repo = StudentRepository::new(pool.inner().clone());
    let student = repo.create(req).map_err(|e| e.to_string())?;
    sync_sf2_roster_after_student_change(pool.inner(), student.class_id.as_deref());
    Ok(student)
}

#[tauri::command]
pub fn create_students(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    reqs: Vec<CreateStudentRequest>,
) -> std::result::Result<Vec<Student>, String> {
    let mut checked_class_ids: Vec<String> = Vec::new();

    for req in &reqs {
        let class_id = req
            .class_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| CREATE_STUDENT_REQUIRES_SF2_CLASS.to_string())?;

        if !checked_class_ids.iter().any(|checked| checked == class_id) {
            ensure_sf2_workbook_exists_for_student_create(pool.inner(), Some(class_id))?;
            checked_class_ids.push(class_id.to_string());
        }
    }

    let repo = StudentRepository::new(pool.inner().clone());
    let students = repo.create_many(reqs).map_err(|e| e.to_string())?;

    for class_id in checked_class_ids {
        sync_sf2_roster_after_student_change(pool.inner(), Some(class_id.as_str()));
    }

    Ok(students)
}

#[tauri::command]
pub fn update_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateStudentRequest,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    let before = repo.get(student_id).map_err(|e| e.to_string())?;
    let student = repo.update(student_id, req).map_err(|e| e.to_string())?;
    if student_roster_fields_changed(&before, &student) {
        sync_sf2_roster_after_student_change(pool.inner(), before.class_id.as_deref());
        if student.class_id != before.class_id {
            sync_sf2_roster_after_student_change(pool.inner(), student.class_id.as_deref());
        }
    }
    Ok(student)
}

#[tauri::command]
pub fn delete_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    let before = repo.get(student_id).map_err(|e| e.to_string())?;
    repo.delete(student_id).map_err(|e| e.to_string())?;
    sync_sf2_roster_after_student_change(pool.inner(), before.class_id.as_deref());
    Ok(())
}
