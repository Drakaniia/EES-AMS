import '../entities/class_entity.dart';
import '../core/result.dart';
import '../exceptions/domain_exceptions.dart';

/// Repository interface for class-related data operations.
/// 
/// This interface defines the contract for class data access without
/// specifying the implementation details, following the Repository pattern.
abstract class ClassRepository {
  /// Retrieves all classes from the system.
  /// 
  /// Returns a list of [ClassEntity] on success.
  Future<Result<List<ClassEntity>>> getAllClasses();

  /// Retrieves a class by its unique identifier.
  /// 
  /// Returns [ClassEntity] if found, returns failure result
  /// with [DataException] if not found.
  Future<Result<ClassEntity>> getClassById(int id);

  /// Creates a new class in the system.
  /// 
  /// Returns created [ClassEntity] with assigned ID on success.
  /// Throws [ValidationException] on invalid data or [DataException] on creation failure.
  Future<Result<ClassEntity>> createClass(ClassEntity classEntity);

  /// Updates an existing class's information.
  /// 
  /// Returns updated [ClassEntity] on success.
  /// Throws [ValidationException] on invalid data or [DataException] on update failure.
  Future<Result<ClassEntity>> updateClass(ClassEntity classEntity);

  /// Deletes a class from the system.
  /// 
  /// Returns success result on deletion,
  /// throws [DataException] if class not found or deletion fails.
  Future<Result<void>> deleteClass(int id);

  /// Retrieves classes filtered by school year.
  /// 
  /// Returns a list of [ClassEntity] matching the school year.
  Future<Result<List<ClassEntity>>> getClassesBySchoolYear(String schoolYear);

  /// Searches for classes based on name or section.
  /// 
  /// [query] can match class name or section.
  /// Returns a list of matching [ClassEntity].
  Future<Result<List<ClassEntity>>> searchClasses(String query);

  /// Gets the number of students enrolled in a class.
  /// 
  /// Returns the student count for the specified class.
  Future<Result<int>> getStudentCount(int classId);
}