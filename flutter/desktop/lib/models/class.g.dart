// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'class.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Class _$ClassFromJson(Map<String, dynamic> json) => Class(
      id: (json['id'] as num?)?.toInt(),
      name: json['name'] as String,
      section: json['section'] as String?,
      schoolYear: json['school_year'] as String?,
      createdAt: json['created_at'] == null
          ? null
          : DateTime.parse(json['created_at'] as String),
      updatedAt: json['updated_at'] == null
          ? null
          : DateTime.parse(json['updated_at'] as String),
    );

Map<String, dynamic> _$ClassToJson(Class instance) => <String, dynamic>{
      'id': instance.id,
      'name': instance.name,
      'section': instance.section,
      'school_year': instance.schoolYear,
      'created_at': instance.createdAt?.toIso8601String(),
      'updated_at': instance.updatedAt?.toIso8601String(),
    };
