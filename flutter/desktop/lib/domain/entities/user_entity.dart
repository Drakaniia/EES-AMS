import 'package:equatable/equatable.dart';

/// Domain entity representing a user in the system.
/// 
/// This entity contains only business logic relevant data and operations
/// without any infrastructure concerns like JSON serialization.
class UserEntity extends Equatable {
  /// Unique identifier for the user.
  final int? id;
  
  /// User's email address.
  final String email;
  
  /// Display name for the user.
  final String displayName;
  
  /// Name of the school the user belongs to (optional).
  final String? schoolName;
  
  /// Role of the user in the system.
  final UserRole role;
  
  /// Whether the user is currently active.
  final bool isActive;
  
  /// When the user was created.
  final DateTime? createdAt;
  
  /// When the user was last updated.
  final DateTime? updatedAt;
  
  /// When the user last logged in.
  final DateTime? lastLogin;

  /// Creates a new [UserEntity].
  const UserEntity({
    this.id,
    required this.email,
    required this.displayName,
    this.schoolName,
    required this.role,
    this.isActive = true,
    this.createdAt,
    this.updatedAt,
    this.lastLogin,
  });

  /// Creates a copy of this [UserEntity] with optional updated values.
  UserEntity copyWith({
    int? id,
    String? email,
    String? displayName,
    String? schoolName,
    UserRole? role,
    bool? isActive,
    DateTime? createdAt,
    DateTime? updatedAt,
    DateTime? lastLogin,
  }) {
    return UserEntity(
      id: id ?? this.id,
      email: email ?? this.email,
      displayName: displayName ?? this.displayName,
      schoolName: schoolName ?? this.schoolName,
      role: role ?? this.role,
      isActive: isActive ?? this.isActive,
      createdAt: createdAt ?? this.createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      lastLogin: lastLogin ?? this.lastLogin,
    );
  }

  /// Returns true if the user has administrator privileges.
  bool get isAdmin => role == UserRole.admin;

  /// Returns true if the user has teacher privileges.
  bool get isTeacher => role == UserRole.teacher;

  /// Returns true if the user is a regular student.
  bool get isStudent => role == UserRole.student;

  @override
  List<Object?> get props => [
        id,
        email,
        displayName,
        schoolName,
        role,
        isActive,
        createdAt,
        updatedAt,
        lastLogin,
      ];
}

/// Enumeration of possible user roles in the system.
enum UserRole {
  /// System administrator with full access.
  admin('admin'),
  
  /// Teacher with class and student management access.
  teacher('teacher'),
  
  /// Student with attendance viewing access.
  student('student');

  /// The string value of the role.
  final String value;

  const UserRole(this.value);

  /// Creates a [UserRole] from a string value.
  /// 
  /// Returns [UserRole.admin] if the string doesn't match any known role.
  static UserRole fromString(String value) {
    switch (value.toLowerCase()) {
      case 'admin':
        return UserRole.admin;
      case 'teacher':
        return UserRole.teacher;
      case 'student':
        return UserRole.student;
      default:
        return UserRole.admin; // Default fallback
    }
  }

  @override
  String toString() => value;
}