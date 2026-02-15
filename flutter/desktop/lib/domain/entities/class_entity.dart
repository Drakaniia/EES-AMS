import 'package:equatable/equatable.dart';
import '../../models/class.dart';

/// Domain entity representing a class in the system.
/// 
/// This entity contains only business logic relevant data and operations
/// without any infrastructure concerns like JSON serialization.
class ClassEntity extends Equatable {
  /// Unique identifier for the class.
  final int? id;
  
  /// Name of the class (e.g., "Mathematics", "Grade 10").
  final String name;
  
  /// Section or subgroup within the class (e.g., "A", "Section 1").
  final String? section;
  
  /// School year the class belongs to (e.g., "2024-2025").
  final String? schoolYear;
  
  /// When the class was created in the system.
  final DateTime? createdAt;
  
  /// When the class was last updated.
  final DateTime? updatedAt;

  /// Creates a new [ClassEntity].
  const ClassEntity({
    this.id,
    required this.name,
    this.section,
    this.schoolYear,
    this.createdAt,
    this.updatedAt,
  });

  /// Creates a copy of this [ClassEntity] with optional updated values.
  ClassEntity copyWith({
    int? id,
    String? name,
    String? section,
    String? schoolYear,
    DateTime? createdAt,
    DateTime? updatedAt,
  }) {
    return ClassEntity(
      id: id ?? this.id,
      name: name ?? this.name,
      section: section ?? this.section,
      schoolYear: schoolYear ?? this.schoolYear,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
    );
  }

  /// Returns the display name combining name, section, and school year.
  /// 
  /// Format: "Name - Section - School Year" (excluding empty parts).
  String get displayName {
    final parts = <String>[name];
    if (section != null && section!.isNotEmpty) parts.add(section!);
    if (schoolYear != null && schoolYear!.isNotEmpty) parts.add(schoolYear!);
    return parts.join(' - ');
  }

  /// Returns true if the class has a section.
  bool get hasSection => section != null && section!.isNotEmpty;

  /// Returns true if the class has a school year specified.
  bool get hasSchoolYear => schoolYear != null && schoolYear!.isNotEmpty;

  /// Validates the class data for creation/update.
  /// 
  /// Returns a list of validation errors, empty if valid.
  List<String> validate() {
    final errors = <String>[];
    
    if (name.trim().isEmpty) {
      errors.add('Class name is required');
    }
    
    if (name.trim().length < 2) {
      errors.add('Class name must be at least 2 characters');
    }
    
    if (section != null && section!.isNotEmpty && section!.length > 10) {
      errors.add('Section must be 10 characters or less');
    }
    
    if (schoolYear != null && schoolYear!.isNotEmpty) {
      final yearRegex = RegExp(r'^\d{4}-\d{4}$');
      if (!yearRegex.hasMatch(schoolYear!)) {
        errors.add('School year must be in format YYYY-YYYY');
      }
    }
    
    return errors;
  }

  /// Creates a ClassEntity from a Class model.
  factory ClassEntity.fromModel(Class model) {
    return ClassEntity(
      id: model.id,
      name: model.name,
      section: model.section,
      schoolYear: model.schoolYear,
      createdAt: model.createdAt,
      updatedAt: model.updatedAt,
    );
  }

  /// Converts this ClassEntity to a Class model.
  Class toModel() {
    return Class(
      id: id,
      name: name,
      section: section,
      schoolYear: schoolYear,
      createdAt: createdAt,
      updatedAt: updatedAt,
    );
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