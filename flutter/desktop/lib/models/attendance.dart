import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';
import 'student.dart';
import 'class.dart';

part 'attendance.g.dart';

enum AttendanceStatus {
  @JsonValue('present')
  present,
  @JsonValue('absent')
  absent,
  @JsonValue('late')
  late,
  @JsonValue('excused')
  excused,
}

@JsonSerializable()
class AttendanceRecord extends Equatable {
  final int? id;
  @JsonKey(name: 'student_id')
  final int studentId;
  @JsonKey(name: 'class_id')
  final int classId;
  @JsonKey(name: 'date')
  final DateTime date;
  @JsonKey(name: 'status')
  final AttendanceStatus status;
  @JsonKey(name: 'notes')
  final String? notes;
  @JsonKey(name: 'created_at')
  final DateTime? createdAt;
  @JsonKey(name: 'updated_at')
  final DateTime? updatedAt;

  // Optional populated relations
  final Student? _student;
  final Class? _classRecord;

  const AttendanceRecord({
    this.id,
    required this.studentId,
    required this.classId,
    required this.date,
    required this.status,
    this.notes,
    this.createdAt,
    this.updatedAt,
    Student? student,
    Class? classRecord,
  })  : _student = student,
        _classRecord = classRecord;

  // Getters for accessing relations
  Student? get student => _student;
  Class? get classRecord => _classRecord;

  factory AttendanceRecord.fromJson(Map<String, dynamic> json) =>
      _$AttendanceRecordFromJson(json);
  Map<String, dynamic> toJson() => _$AttendanceRecordToJson(this);

  AttendanceRecord copyWith({
    int? id,
    int? studentId,
    int? classId,
    DateTime? date,
    AttendanceStatus? status,
    String? notes,
    DateTime? createdAt,
    DateTime? updatedAt,
    Student? student,
    Class? classRecord,
  }) {
    return AttendanceRecord(
      id: id ?? this.id,
      studentId: studentId ?? this.studentId,
      classId: classId ?? this.classId,
      date: date ?? this.date,
      status: status ?? this.status,
      notes: notes ?? this.notes,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      student: student ?? _student,
      classRecord: classRecord ?? _classRecord,
    );
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
        _student,
        _classRecord,
      ];
}

@JsonSerializable()
class AttendanceStats extends Equatable {
  @JsonKey(name: 'total_students')
  final int totalStudents;
  @JsonKey(name: 'present_today')
  final int presentToday;
  @JsonKey(name: 'absent_today')
  final int absentToday;
  @JsonKey(name: 'late_today')
  final int lateToday;
  @JsonKey(name: 'excused_today')
  final int excusedToday;
  @JsonKey(name: 'attendance_rate')
  final double attendanceRate;

  const AttendanceStats({
    required this.totalStudents,
    required this.presentToday,
    required this.absentToday,
    required this.lateToday,
    required this.excusedToday,
    required this.attendanceRate,
  });

  factory AttendanceStats.fromJson(Map<String, dynamic> json) =>
      _$AttendanceStatsFromJson(json);
  Map<String, dynamic> toJson() => _$AttendanceStatsToJson(this);

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
