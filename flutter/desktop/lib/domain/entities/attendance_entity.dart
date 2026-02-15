import 'package:equatable/equatable.dart';
import '../../models/attendance.dart' as model;

/// Enumeration of possible attendance statuses.
enum AttendanceStatus {
  /// Student was present in class.
  present('present'),

  /// Student was absent from class.
  absent('absent'),

  /// Student arrived late to class.
  late('late'),

  /// Student was excused from class.
  excused('excused');

  /// The string value of the status.
  final String value;

  const AttendanceStatus(this.value);

  /// Creates an [AttendanceStatus] from a string value.
  ///
  /// Returns [AttendanceStatus.present] if the string doesn't match any known status.
  static AttendanceStatus fromString(String value) {
    switch (value.toLowerCase()) {
      case 'present':
        return AttendanceStatus.present;
      case 'absent':
        return AttendanceStatus.absent;
      case 'late':
        return AttendanceStatus.late;
      case 'excused':
        return AttendanceStatus.excused;
      default:
        return AttendanceStatus.present; // Default fallback
    }
  }

  @override
  String toString() => value;
}

/// Domain entity representing an attendance record.
///
/// This entity contains only business logic relevant data and operations
/// without any infrastructure concerns like JSON serialization.
class AttendanceRecordEntity extends Equatable {
  /// Unique identifier for the attendance record.
  final int? id;

  /// ID of the student.
  final int studentId;

  /// ID of the class.
  final int classId;

  /// Date of the attendance record.
  final DateTime date;

  /// Attendance status for the record.
  final AttendanceStatus status;

  /// Optional notes about the attendance record.
  final String? notes;

  /// When the record was created.
  final DateTime? createdAt;

  /// When the record was last updated.
  final DateTime? updatedAt;

  /// Creates a new [AttendanceRecordEntity].
  const AttendanceRecordEntity({
    this.id,
    required this.studentId,
    required this.classId,
    required this.date,
    required this.status,
    this.notes,
    this.createdAt,
    this.updatedAt,
  });

  /// Creates a copy of this [AttendanceRecordEntity] with optional updated values.
  AttendanceRecordEntity copyWith({
    int? id,
    int? studentId,
    int? classId,
    DateTime? date,
    AttendanceStatus? status,
    String? notes,
    DateTime? createdAt,
    DateTime? updatedAt,
  }) {
    return AttendanceRecordEntity(
      id: id ?? this.id,
      studentId: studentId ?? this.studentId,
      classId: classId ?? this.classId,
      date: date ?? this.date,
      status: status ?? this.status,
      notes: notes ?? this.notes,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  /// Returns true if the student was present (present or late).
  bool get isPresent =>
      status == AttendanceStatus.present || status == AttendanceStatus.late;

  /// Returns true if the student was absent (absent or excused).
  bool get isAbsent =>
      status == AttendanceStatus.absent || status == AttendanceStatus.excused;

  /// Returns true if the record has notes.
  bool get hasNotes => notes != null && notes!.isNotEmpty;

  /// Creates an AttendanceRecordEntity from an AttendanceRecord model.
  factory AttendanceRecordEntity.fromModel(model.AttendanceRecord model) {
    return AttendanceRecordEntity(
      id: model.id,
      studentId: model.studentId,
      classId: model.classId,
      date: model.date,
      status: AttendanceStatus.fromString(model.status.name),
      notes: model.notes,
      createdAt: model.createdAt,
      updatedAt: model.updatedAt,
    );
  }

  /// Converts this AttendanceRecordEntity to an AttendanceRecord model.
  model.AttendanceRecord toModel() {
    return model.AttendanceRecord(
      id: id,
      studentId: studentId,
      classId: classId,
      date: date,
      status: _toModelStatus(status),
      notes: notes,
      createdAt: createdAt,
      updatedAt: updatedAt,
    );
  }

  model.AttendanceStatus _toModelStatus(AttendanceStatus status) {
    switch (status) {
      case AttendanceStatus.present:
        return model.AttendanceStatus.present;
      case AttendanceStatus.absent:
        return model.AttendanceStatus.absent;
      case AttendanceStatus.late:
        return model.AttendanceStatus.late;
      case AttendanceStatus.excused:
        return model.AttendanceStatus.excused;
    }
  }

  @override
  List<Object?> get props => [
        id,
        studentId,
        classId,
        date,
        status,
        notes,
        createdAt,
        updatedAt,
      ];
}

/// Domain entity representing attendance statistics.
///
/// This entity contains aggregated attendance data for reporting.
class AttendanceStatsEntity extends Equatable {
  /// Total number of students.
  final int totalStudents;

  /// Number of students present today.
  final int presentToday;

  /// Number of students absent today.
  final int absentToday;

  /// Number of students late today.
  final int lateToday;

  /// Number of excused absences today.
  final int excusedToday;

  /// Attendance rate as a percentage (0.0 to 1.0).
  final double attendanceRate;

  /// Creates a new [AttendanceStatsEntity].
  const AttendanceStatsEntity({
    required this.totalStudents,
    required this.presentToday,
    required this.absentToday,
    required this.lateToday,
    required this.excusedToday,
    required this.attendanceRate,
  });

  /// Returns the number of students accounted for.
  int get accountedForStudents =>
      presentToday + absentToday + lateToday + excusedToday;

  /// Returns the number of unaccounted students.
  int get unaccountedStudents => totalStudents - accountedForStudents;

  /// Returns true if all students are accounted for.
  bool get isComplete => unaccountedStudents == 0;

  /// Returns the attendance rate as a percentage string.
  String get attendanceRatePercentage =>
      '${(attendanceRate * 100).toStringAsFixed(1)}%';

  /// Creates an AttendanceStatsEntity from an AttendanceStats model.
  factory AttendanceStatsEntity.fromModel(model.AttendanceStats stats) {
    return AttendanceStatsEntity(
      totalStudents: stats.totalStudents,
      presentToday: stats.presentToday,
      absentToday: stats.absentToday,
      lateToday: stats.lateToday,
      excusedToday: stats.excusedToday,
      attendanceRate: stats.attendanceRate,
    );
  }

  /// Converts this AttendanceStatsEntity to an AttendanceStats model.
  model.AttendanceStats toModel() {
    return model.AttendanceStats(
      totalStudents: totalStudents,
      presentToday: presentToday,
      absentToday: absentToday,
      lateToday: lateToday,
      excusedToday: excusedToday,
      attendanceRate: attendanceRate,
    );
  }

  @override
  List<Object?> get props => [
        totalStudents,
        presentToday,
        absentToday,
        lateToday,
        excusedToday,
        attendanceRate,
      ];
}
