// Domain Entity: Student
// Represents a student in the attendance management system

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a student
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Student {
    pub id: i64,
    pub student_id: String,
    pub lrn: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub gender: Option<String>,
    pub birthday: Option<String>,
    pub age: Option<i32>,
    pub mother_name: Option<String>,
    pub father_name: Option<String>,
    pub guardian_name: Option<String>,
    pub address: Option<String>,
    pub class_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl Student {
    #[allow(dead_code)]
    pub fn new(
        id: i64,
        student_id: String,
        first_name: String,
        last_name: String,
        class_id: Option<i64>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Student {
            id,
            student_id,
            lrn: None,
            first_name,
            last_name,
            middle_name: None,
            gender: None,
            birthday: None,
            age: None,
            mother_name: None,
            father_name: None,
            guardian_name: None,
            address: None,
            class_id,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn new_from_sf1(
        id: i64,
        lrn: Option<String>,
        last_name: String,
        first_name: String,
        middle_name: Option<String>,
        gender: Option<String>,
        birthday: Option<String>,
        age: Option<i32>,
        mother_name: Option<String>,
        father_name: Option<String>,
        guardian_name: Option<String>,
        address: Option<String>,
        class_id: Option<i64>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let student_id = lrn.clone().unwrap_or_else(|| format!("STD{:06}", id));
        Student {
            id,
            student_id,
            lrn,
            last_name,
            first_name,
            middle_name,
            gender,
            birthday,
            age,
            mother_name,
            father_name,
            guardian_name,
            address,
            class_id,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[allow(dead_code)]
    pub fn full_name(&self) -> String {
        match &self.middle_name {
            Some(middle) => format!("{} {} {}", self.first_name, middle, self.last_name),
            None => format!("{} {}", self.first_name, self.last_name),
        }
    }

    #[allow(dead_code)]
    pub fn full_name_lfmi(&self) -> String {
        match &self.middle_name {
            Some(middle) => format!("{}, {} {}", self.last_name, self.first_name, middle),
            None => format!("{}, {}", self.last_name, self.first_name),
        }
    }

    #[allow(dead_code)]
    pub fn update(
        &mut self,
        rfid: Option<String>,
        lrn: Option<String>,
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        gender: String,
        birthday: String, // YYYY-MM-DD
        age: i32,
        mother_name: Option<String>,
        father_name: Option<String>,
        guardian_name: Option<String>,
        address: Option<String>,
        class_id: Option<i64>,
    ) {
        // This method body was not provided in the instruction,
        // so keeping the original logic for updating fields based on new parameters.
        // Assuming the intent was to update the student's fields with the new values.
        if let Some(rfid_val) = rfid {
            self.student_id = rfid_val; // Assuming rfid updates student_id
        }
        if let Some(lrn_val) = lrn {
            self.lrn = Some(lrn_val);
        }
        self.first_name = first_name;
        self.last_name = last_name;
        self.middle_name = middle_name;
        self.gender = Some(gender);
        self.birthday = Some(birthday);
        self.age = Some(age);
        self.mother_name = mother_name;
        self.father_name = father_name;
        self.guardian_name = guardian_name;
        self.address = address;
        self.class_id = class_id;

        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    #[allow(dead_code)]
    pub fn is_assigned_to_class(&self, _class_id: i64) -> bool {
        self.class_id == Some(_class_id)
    }
}
