import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'class.g.dart';

@JsonSerializable()
class Class extends Equatable {
  final int? id;
  @JsonKey(name: 'name')
  final String name;
  @JsonKey(name: 'section')
  final String? section;
  @JsonKey(name: 'school_year')
  final String? schoolYear;
  @JsonKey(name: 'created_at')
  final DateTime? createdAt;
  @JsonKey(name: 'updated_at')
  final DateTime? updatedAt;

  const Class({
    this.id,
    required this.name,
    this.section,
    this.schoolYear,
    this.createdAt,
    this.updatedAt,
  });

  factory Class.fromJson(Map<String, dynamic> json) => _$ClassFromJson(json);
  Map<String, dynamic> toJson() => _$ClassToJson(this);

  Class copyWith({
    int? id,
    String? name,
    String? section,
    String? schoolYear,
    DateTime? createdAt,
    DateTime? updatedAt,
  }) {
    return Class(
      id: id ?? this.id,
      name: name ?? this.name,
      section: section ?? this.section,
      schoolYear: schoolYear ?? this.schoolYear,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  String get displayName {
    final parts = <String>[name];
    if (section != null && section!.isNotEmpty) parts.add(section!);
    if (schoolYear != null && schoolYear!.isNotEmpty) parts.add(schoolYear!);
    return parts.join(' - ');
  }

  @override
  List<Object?> get props => [
        id,
        name,
        section,
        schoolYear,
        createdAt,
        updatedAt,
      ];
}