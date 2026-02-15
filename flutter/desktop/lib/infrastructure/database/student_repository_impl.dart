import 'package:shared_preferences/shared_preferences.dart';
import '../../services/student_service.dart';
import '../../domain/entities/student_entity.dart';
import '../../domain/core/result.dart';
import '../../domain/exceptions/domain_exceptions.dart';
import '../../domain/repositories/student_repository.dart';

/// Concrete implementation of the StudentRepository interface.
///
/// This implementation uses the StudentService to communicate with the backend
/// API and handles the mapping between domain entities and DTOs.
class StudentRepositoryImpl implements StudentRepository {
  final StudentService _studentService;
  SharedPreferences? _sharedPreferences;

  StudentRepositoryImpl(this._studentService);

  Future<void> _initPrefs() async {
    _sharedPreferences ??= await SharedPreferences.getInstance();
  }

  Future<String> _getAuthToken() async {
    try {
      await _initPrefs();
      // Try to get token from shared preferences
      final token = _sharedPreferences?.getString('auth_token');

      if (token != null && token.isNotEmpty) {
        return 'Bearer $token';
      }

      // Return empty string if no token found
      return '';
    } catch (e) {
      // Log the error or handle appropriately
      // Log the error or handle appropriately
      // print('Error retrieving auth token: $e');
      return '';
    }
  }

  Future<void> saveAuthToken(String token) async {
    try {
      await _initPrefs();
      await _sharedPreferences?.setString('auth_token', token);
    } catch (e) {
      // Log the error or handle appropriately
      // print('Error saving auth token: $e');
      rethrow;
    }
  }

  Future<void> clearAuthToken() async {
    try {
      await _initPrefs();
      await _sharedPreferences?.remove('auth_token');
    } catch (e) {
      // Log the error or handle appropriately
      // print('Error clearing auth token: $e');
    }
  }

  @override
  Future<Result<List<StudentEntity>>> getAllStudents() async {
    try {
      final authToken = await _getAuthToken();
      final response = await _studentService.getAllStudents(authToken);

      if (response.success && response.data != null) {
        final students = response.data!
            .map((student) => StudentEntity.fromModel(student))
            .toList();
        return Result.success(students);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to fetch students'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<StudentEntity>> getStudentById(int id) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _studentService.getStudentById(authToken, id);

      if (response.success && response.data != null) {
        final student = StudentEntity.fromModel(response.data!);
        return Result.success(student);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Student not found'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<StudentEntity>> getStudentByStudentId(String studentId) async {
    try {
      // Use search endpoint to find by student ID
      final authToken = await _getAuthToken();
      final response =
          await _studentService.searchStudents(authToken, studentId, null);

      if (response.success &&
          response.data != null &&
          response.data!.isNotEmpty) {
        final student = StudentEntity.fromModel(response.data!.first);
        return Result.success(student);
      } else {
        return Result.failure(
            DataException('Student with ID $studentId not found'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<List<StudentEntity>>> getStudentsByClassId(int classId) async {
    try {
      final authToken = await _getAuthToken();
      final response =
          await _studentService.getStudentsByClass(authToken, classId);

      if (response.success && response.data != null) {
        final students = response.data!
            .map((student) => StudentEntity.fromModel(student))
            .toList();
        return Result.success(students);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to fetch students for class'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<StudentEntity>> createStudent(StudentEntity student) async {
    try {
      // Validation
      final validationErrors = student.validate();
      if (validationErrors.isNotEmpty) {
        return Result.failure(ValidationException(validationErrors.join(', ')));
      }

      final studentData = student.toModel().toJson();
      final authToken = await _getAuthToken();
      final response =
          await _studentService.createStudent(authToken, studentData);

      if (response.success && response.data != null) {
        final createdStudent = StudentEntity.fromModel(response.data!);
        return Result.success(createdStudent);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to create student'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<StudentEntity>> updateStudent(StudentEntity student) async {
    try {
      if (student.id == null) {
        return Result.failure(
            DataException('Student ID is required for update'));
      }

      // Validation
      final validationErrors = student.validate();
      if (validationErrors.isNotEmpty) {
        return Result.failure(ValidationException(validationErrors.join(', ')));
      }

      final studentData = student.toModel().toJson();
      final authToken = await _getAuthToken();
      final response = await _studentService.updateStudent(
          authToken, student.id!, studentData);

      if (response.success && response.data != null) {
        final updatedStudent = StudentEntity.fromModel(response.data!);
        return Result.success(updatedStudent);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to update student'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<void>> deleteStudent(int id) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _studentService.deleteStudent(authToken, id);

      if (response.success) {
        return Result.success(null);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to delete student'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<StudentEntity>> assignStudentToClass({
    required int studentId,
    required int classId,
  }) async {
    try {
      // Get the student first
      final getResult = await getStudentById(studentId);
      if (getResult.isFailure) {
        return Result.failure(getResult.error!);
      }

      final student = getResult.data!;
      final updatedStudent = student.copyWith(classId: classId);

      return await updateStudent(updatedStudent);
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<StudentEntity>> removeStudentFromClass(int studentId) async {
    try {
      // Get the student first
      final getResult = await getStudentById(studentId);
      if (getResult.isFailure) {
        return Result.failure(getResult.error!);
      }

      final student = getResult.data!;
      final updatedStudent = student.copyWith(classId: null);

      return await updateStudent(updatedStudent);
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<List<StudentEntity>>> searchStudents({
    required String query,
    int? classId,
  }) async {
    try {
      final authToken = await _getAuthToken();
      final response =
          await _studentService.searchStudents(authToken, query, classId);

      if (response.success && response.data != null) {
        final students = response.data!
            .map((student) => StudentEntity.fromModel(student))
            .toList();
        return Result.success(students);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to search students'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<List<StudentEntity>>> importStudentsFromExcel(
      List<Map<String, dynamic>> excelData) async {
    try {
      final authToken = await _getAuthToken();
      final response =
          await _studentService.createBulkStudents(authToken, excelData);

      if (response.success && response.data != null) {
        final students = response.data!
            .map((student) => StudentEntity.fromModel(student))
            .toList();
        return Result.success(students);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to import students from Excel'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  @override
  Future<Result<List<Map<String, dynamic>>>> exportStudentsToExcel(
      {int? classId}) async {
    try {
      // This would need to be implemented as a separate endpoint or handled differently
      // For now, return the student data in a format that can be exported
      final studentResult = classId != null
          ? await getStudentsByClassId(classId)
          : await getAllStudents();

      if (studentResult.isSuccess) {
        final exportData = studentResult.data!
            .map((student) => {
                  'student_id': student.studentId,
                  'first_name': student.firstName,
                  'last_name': student.lastName,
                  'email': student.email,
                  'phone': student.phone,
                  'class_id': student.classId,
                })
            .toList();

        return Result.success(exportData);
      } else {
        return Result.failure(studentResult.error!);
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }
}
