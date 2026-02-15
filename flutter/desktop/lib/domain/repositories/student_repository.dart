import '../entities/student_entity.dart';
import '../core/result.dart';
import '../exceptions/domain_exceptions.dart';

/// Repository interface for student-related data operations.
/// 
/// This interface defines the contract for student data access without
/// specifying the implementation details, following the Repository pattern.
abstract class StudentRepository {
  /// Retrieves all students from the system.
  /// 
  /// Returns a list of [StudentEntity] on success.
  Future<Result<List<StudentEntity>>> getAllStudents();

  /// Retrieves a student by their unique identifier.
  /// 
  /// Returns [StudentEntity] if found, returns failure result
  /// with [DataException] if not found.
  Future<Result<StudentEntity>> getStudentById(int id);

  /// Retrieves a student by their student ID number.
  /// 
  /// Returns [StudentEntity] if found, returns failure result
  /// with [DataException] if not found.
  Future<Result<StudentEntity>> getStudentByStudentId(String studentId);

  /// Retrieves all students assigned to a specific class.
  /// 
  /// Returns a list of [StudentEntity] on success.
  Future<Result<List<StudentEntity>>> getStudentsByClassId(int classId);

  /// Creates a new student in the system.
  /// 
  /// Returns created [StudentEntity] with assigned ID on success.
  /// Throws [ValidationException] on invalid data or [DataException] on creation failure.
  Future<Result<StudentEntity>> createStudent(StudentEntity student);

  /// Updates an existing student's information.
  /// 
  /// Returns updated [StudentEntity] on success.
  /// Throws [ValidationException] on invalid data or [DataException] on update failure.
  Future<Result<StudentEntity>> updateStudent(StudentEntity student);

  /// Deletes a student from the system.
  /// 
  /// Returns success result on deletion,
  /// throws [DataException] if student not found or deletion fails.
  Future<Result<void>> deleteStudent(int id);

  /// Assigns a student to a class.
  /// 
  /// Returns updated [StudentEntity] on success,
  /// throws [DataException] if assignment fails.
  Future<Result<StudentEntity>> assignStudentToClass({
    required int studentId,
    required int classId,
  });

  /// Removes a student from their assigned class.
  /// 
  /// Returns updated [StudentEntity] on success,
  /// throws [DataException] if removal fails.
  Future<Result<StudentEntity>> removeStudentFromClass(int studentId);

  /// Searches for students based on criteria.
  /// 
  /// [query] can match first name, last name, or student ID.
  /// [classId] is optional to filter by class.
  /// Returns a list of matching [StudentEntity].
  Future<Result<List<StudentEntity>>> searchStudents({
    required String query,
    int? classId,
  });

  /// Imports students from Excel data.
  /// 
  /// Returns a list of created [StudentEntity] on success.
  /// Throws [ValidationException] on invalid data format or [DataException] on import failure.
  Future<Result<List<StudentEntity>>> importStudentsFromExcel(List<Map<String, dynamic>> excelData);

  /// Exports students to Excel format.
  /// 
  /// [classId] is optional to filter by class.
  /// Returns Excel data as a list of maps.
  Future<Result<List<Map<String, dynamic>>>> exportStudentsToExcel({int? classId});
}