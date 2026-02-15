// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'attendance.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AttendanceRecord _$AttendanceRecordFromJson(Map<String, dynamic> json) =>
    AttendanceRecord(
      id: (json['id'] as num?)?.toInt(),
      studentId: (json['student_id'] as num).toInt(),
      classId: (json['class_id'] as num).toInt(),
      date: DateTime.parse(json['date'] as String),
      status: $enumDecode(_$AttendanceStatusEnumMap, json['status']),
      notes: json['notes'] as String?,
      createdAt: json['created_at'] == null
          ? null
          : DateTime.parse(json['created_at'] as String),
      updatedAt: json['updated_at'] == null
          ? null
          : DateTime.parse(json['updated_at'] as String),
      student: json['student'] == null
          ? null
          : Student.fromJson(json['student'] as Map<String, dynamic>),
      classRecord: json['classRecord'] == null
          ? null
          : Class.fromJson(json['classRecord'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$AttendanceRecordToJson(AttendanceRecord instance) =>
    <String, dynamic>{
      'id': instance.id,
      'student_id': instance.studentId,
      'class_id': instance.classId,
      'date': instance.date.toIso8601String(),
      'status': _$AttendanceStatusEnumMap[instance.status]!,
      'notes': instance.notes,
      'created_at': instance.createdAt?.toIso8601String(),
      'updated_at': instance.updatedAt?.toIso8601String(),
      'student': instance.student,
      'classRecord': instance.classRecord,
    };

const _$AttendanceStatusEnumMap = {
  AttendanceStatus.present: 'present',
  AttendanceStatus.absent: 'absent',
  AttendanceStatus.late: 'late',
  AttendanceStatus.excused: 'excused',
};

AttendanceStats _$AttendanceStatsFromJson(Map<String, dynamic> json) =>
    AttendanceStats(
      totalStudents: (json['total_students'] as num).toInt(),
      presentToday: (json['present_today'] as num).toInt(),
      absentToday: (json['absent_today'] as num).toInt(),
      lateToday: (json['late_today'] as num).toInt(),
      excusedToday: (json['excused_today'] as num).toInt(),
      attendanceRate: (json['attendance_rate'] as num).toDouble(),
    );

Map<String, dynamic> _$AttendanceStatsToJson(AttendanceStats instance) =>
    <String, dynamic>{
      'total_students': instance.totalStudents,
      'present_today': instance.presentToday,
      'absent_today': instance.absentToday,
      'late_today': instance.lateToday,
      'excused_today': instance.excusedToday,
      'attendance_rate': instance.attendanceRate,
    };
