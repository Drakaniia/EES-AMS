import 'package:equatable/equatable.dart';
import '../../models/student.dart';

/// Domain entity representing a student in the system.
/// 
/// This entity contains only business logic relevant data and operations
/// without any infrastructure concerns like JSON serialization.
class StudentEntity extends Equatable {
  /// Unique identifier for the student.
  final int? id;
  
  /// Student ID number (usually from school records).
  final String studentId;
  
  /// Student's first name.
  final String firstName;
  
  /// Student's last name.
  final String lastName;
  
  /// ID of the class the student belongs to.
  final int? classId;
  
  /// Student's email address (optional).
  final String? email;
  
  /// Student's phone number (optional).
  final String? phone;
  
  /// When the student was created in the system.
  final DateTime? createdAt;
  
  /// When the student was last updated.
  final DateTime? updatedAt;

  /// Creates a new [StudentEntity].
  const StudentEntity({
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

  /// Creates a copy of this [StudentEntity] with optional updated values.
  StudentEntity copyWith({
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
    return StudentEntity(
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

  /// Returns the student's full name in "Last, First" format.
  String get fullName => '$lastName, $firstName';

  /// Returns the student's display name in "First Last" format.
  String get displayName => '$firstName $lastName';

  /// Returns true if the student is assigned to a class.
  bool get hasClass => classId != null;

  /// Returns true if the student has contact information.
  bool get hasContactInfo => email != null || phone != null;

  /// Validates the student data for creation/update.
  /// 
  /// Returns a list of validation errors, empty if valid.
  List<String> validate() {
    final errors = <String>[];
    
    if (studentId.trim().isEmpty) {
      errors.add('Student ID is required');
    }
    
    if (firstName.trim().isEmpty) {
      errors.add('First name is required');
    }
    
    if (lastName.trim().isEmpty) {
      errors.add('Last name is required');
    }
    
    if (email != null && email!.isNotEmpty) {
      final emailRegex = RegExp(r'^[^@]+@[^@]+\.[^@]+$');
      if (!emailRegex.hasMatch(email!)) {
        errors.add('Invalid email format');
      }
    }
    
    if (errors.isNotEmpty) {
      return errors;
    }
    return [];
  }

  /// Creates a StudentEntity from a Student model.
  factory StudentEntity.fromModel(Student model) {
    return StudentEntity(
      id: model.id,
      studentId: model.studentId,
      firstName: model.firstName,
      lastName: model.lastName,
      classId: model.classId,
      email: model.email,
      phone: model.phone,
      createdAt: model.createdAt,
      updatedAt: model.updatedAt,
    );
  }

  /// Converts this StudentEntity to a Student model.
  Student toModel() {
    return Student(
      id: id,
      studentId: studentId,
      firstName: firstName,
      lastName: lastName,
      classId: classId,
      email: email,
      phone: phone,
      createdAt: createdAt,
      updatedAt: updatedAt,
    );
  }

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