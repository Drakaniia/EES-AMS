// Attendance Repository Implementation
// JSON file-based implementation of AttendanceRepository trait

use async_trait::async_trait;
use crate::domain::entities::attendance::{Attendance, AttendanceStatus, AttendanceStats};
use crate::domain::repositories::AttendanceRepository;
use crate::domain::errors::{DomainError, DomainResult};
use crate::infrastructure::database::schema::AttendanceRecord;
use crate::infrastructure::database::JsonDatabase;

pub struct AttendanceRepositoryImpl {
    db: JsonDatabase,
}

impl AttendanceRepositoryImpl {
    pub fn new(db: JsonDatabase) -> Self {
        AttendanceRepositoryImpl { db }
    }

    fn record_to_entity(record: &AttendanceRecord) -> Attendance {
        Attendance {
            id: record.id,
            student_id: record.student_id,
            class_id: record.class_id,
            date: record.date.clone(),
            status: AttendanceStatus::from_str(&record.status),
            notes: record.notes.clone(),
            synced: record.synced,
            created_at: record.created_at.clone(),
        }
    }

    fn entity_to_record(entity: &Attendance) -> AttendanceRecord {
        AttendanceRecord {
            id: entity.id,
            student_id: entity.student_id,
            class_id: entity.class_id,
            date: entity.date.clone(),
            status: entity.status.as_str().to_string(),
            notes: entity.notes.clone(),
            synced: entity.synced,
            created_at: entity.created_at.clone(),
        }
    }
}

#[async_trait]
impl AttendanceRepository for AttendanceRepositoryImpl {
    async fn record(
        &self,
        student_id: i64,
        class_id: i64,
        date: String,
        status: AttendanceStatus,
        notes: Option<String>,
    ) -> DomainResult<i64> {
        let mut data = self.db.get_data().lock().unwrap();
        
        if let Some(existing) = data.attendance.iter_mut().find(|a| {
            a.student_id == student_id && a.class_id == class_id && a.date == date
        }) {
            existing.status = status.as_str().to_string();
            existing.notes = notes;
            existing.synced = false;
            let id = existing.id;
            drop(data);
            self.db.save()?;
            return Ok(id);
        }

        let now = chrono::Utc::now().to_rfc3339();
        data.counters.attendance += 1;
        let id = data.counters.attendance;

        let record = AttendanceRecord {
            id,
            student_id,
            class_id,
            date,
            status: status.as_str().to_string(),
            notes,
            synced: false,
            created_at: now,
        };

        data.attendance.push(record);
        drop(data);
        
        self.db.save()?;
        Ok(id)
    }

    async fn get_by_id(&self, id: i64) -> DomainResult<Attendance> {
        let data = self.db.get_data().lock().unwrap();
        data.attendance.iter()
            .find(|a| a.id == id)
            .map(Self::record_to_entity)
            .ok_or_else(|| DomainError::NotFound(format!("Attendance with id {} not found", id)))
    }

    async fn get_by_class_and_date(&self, class_id: i64, date: &str) -> DomainResult<Vec<Attendance>> {
        let data = self.db.get_data().lock().unwrap();
        let records: Vec<Attendance> = data.attendance.iter()
            .filter(|a| a.class_id == class_id && a.date == date)
            .map(Self::record_to_entity)
            .collect();
        Ok(records)
    }

    async fn get_unsynced(&self) -> DomainResult<Vec<Attendance>> {
        let data = self.db.get_data().lock().unwrap();
        let records: Vec<Attendance> = data.attendance.iter()
            .filter(|a| !a.synced)
            .map(Self::record_to_entity)
            .collect();
        Ok(records)
    }

    async fn get_unsynced_by_class(&self, class_id: i64) -> DomainResult<Vec<Attendance>> {
        let data = self.db.get_data().lock().unwrap();
        let records: Vec<Attendance> = data.attendance.iter()
            .filter(|a| !a.synced && a.class_id == class_id)
            .map(Self::record_to_entity)
            .collect();
        Ok(records)
    }

    async fn mark_as_synced(&self, record_ids: Vec<i64>) -> DomainResult<()> {
        if record_ids.is_empty() {
            return Ok(());
        }

        let mut data = self.db.get_data().lock().unwrap();
        for record in data.attendance.iter_mut() {
            if record_ids.contains(&record.id) {
                record.synced = true;
            }
        }
        drop(data);
        
        self.db.save()?;
        Ok(())
    }

    async fn get_stats(&self, class_id: i64, date: &str) -> DomainResult<AttendanceStats> {
        let data = self.db.get_data().lock().unwrap();
        
        let total_students = data.students.iter()
            .filter(|s| s.class_id == Some(class_id))
            .count() as i32;

        let mut stats = AttendanceStats::new(total_students);

        data.attendance.iter()
            .filter(|a| a.class_id == class_id && a.date == date)
            .for_each(|a| {
                match a.status.to_lowercase().as_str() {
                    "present" => stats.present_count += 1,
                    "absent" => stats.absent_count += 1,
                    "late" => stats.late_count += 1,
                    "excused" => stats.excused_count += 1,
                    _ => {}
                }
            });

        stats.calculate_rate();
        Ok(stats)
    }

    async fn get_by_student_and_date_range(
        &self,
        student_id: i64,
        start_date: &str,
        end_date: &str,
    ) -> DomainResult<Vec<Attendance>> {
        let data = self.db.get_data().lock().unwrap();
        let records: Vec<Attendance> = data.attendance.iter()
            .filter(|a| a.student_id == student_id && a.date >= start_date && a.date <= end_date)
            .map(Self::record_to_entity)
            .collect();
        Ok(records)
    }

    async fn exists(&self, student_id: i64, class_id: i64, date: &str) -> DomainResult<bool> {
        let data = self.db.get_data().lock().unwrap();
        Ok(data.attendance.iter()
            .any(|a| a.student_id == student_id && a.class_id == class_id && a.date == date))
    }
}