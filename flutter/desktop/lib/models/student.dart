import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'student.g.dart';

@JsonSerializable()
class Student extends Equatable {
  final int? id;
  @JsonKey(name: 'student_id')
  final String studentId;
  @JsonKey(name: 'first_name')
  final String firstName;
  @JsonKey(name: 'last_name')
  final String lastName;
  @JsonKey(name: 'class_id')
  final int? classId;
  @JsonKey(name: 'email')
  final String? email;
  @JsonKey(name: 'phone')
  final String? phone;
  @JsonKey(name: 'created_at')
  final DateTime? createdAt;
  @JsonKey(name: 'updated_at')
  final DateTime? updatedAt;

  const Student({
    this.id,
    required this.studentId,
    required this.firstName,
    required this.lastName,
    this.classId,
    this.email,
    this.phone,
    this.createdAt,
    this.updatedAt,
  });

  factory Student.fromJson(Map<String, dynamic> json) => _$StudentFromJson(json);
  Map<String, dynamic> toJson() => _$StudentToJson(this);

  Student copyWith({
    int? id,
    String? studentId,
    String? firstName,
    String? lastName,
    int? classId,
    String? email,
    String? phone,
    DateTime? createdAt,
    DateTime? updatedAt,
  }) {
    return Student(
      id: id ?? this.id,
      studentId: studentId ?? this.studentId,
      firstName: firstName ?? this.firstName,
      lastName: lastName ?? this.lastName,
      classId: classId ?? this.classId,
      email: email ?? this.email,
      phone: phone ?? this.phone,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  String get fullName => '$lastName, $firstName';

  String get displayName => '$firstName $lastName';

  @override
  List<Object?> get props => [
        id,
        studentId,
        firstName,
        lastName,
        classId,
        email,
        phone,
        createdAt,
        updatedAt,
      ];
}