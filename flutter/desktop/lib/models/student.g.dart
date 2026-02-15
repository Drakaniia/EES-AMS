// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'student.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Student _$StudentFromJson(Map<String, dynamic> json) => Student(
      id: (json['id'] as num?)?.toInt(),
      studentId: json['student_id'] as String,
      firstName: json['first_name'] as String,
      lastName: json['last_name'] as String,
      classId: (json['class_id'] as num?)?.toInt(),
      email: json['email'] as String?,
      phone: json['phone'] as String?,
      createdAt: json['created_at'] == null
          ? null
          : DateTime.parse(json['created_at'] as String),
      updatedAt: json['updated_at'] == null
          ? null
          : DateTime.parse(json['updated_at'] as String),
    );

Map<String, dynamic> _$StudentToJson(Student instance) => <String, dynamic>{
      'id': instance.id,
      'student_id': instance.studentId,
      'first_name': instance.firstName,
      'last_name': instance.lastName,
      'class_id': instance.classId,
      'email': instance.email,
      'phone': instance.phone,
      'created_at': instance.createdAt?.toIso8601String(),
      'updated_at': instance.updatedAt?.toIso8601String(),
    };
