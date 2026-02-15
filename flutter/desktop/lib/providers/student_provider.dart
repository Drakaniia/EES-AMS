import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import '../models/student.dart';
import '../services/student_service.dart';
import '../services/api_client.dart';

class StudentState {
  final List<Student> students;
  final Student? selectedStudent;
  final bool isLoading;
  final String? error;

  const StudentState({
    this.students = const [],
    this.selectedStudent,
    this.isLoading = false,
    this.error,
  });

  StudentState copyWith({
    List<Student>? students,
    Student? selectedStudent,
    bool? isLoading,
    String? error,
  }) {
    return StudentState(
      students: students ?? this.students,
      selectedStudent: selectedStudent ?? this.selectedStudent,
      isLoading: isLoading ?? this.isLoading,
      error: error ?? this.error,
    );
  }
}

final studentProvider =
    StateNotifierProvider<StudentNotifier, StudentState>((ref) {
  final studentService = ref.read(studentServiceProvider);
  return StudentNotifier(studentService);
});

class StudentNotifier extends StateNotifier<StudentState> {
  final StudentService _studentService;
  final Logger _logger;

  StudentNotifier(this._studentService, [Logger? logger])
      : _logger = logger ?? Logger(),
        super(const StudentState());

  Future<void> loadStudents({int? classId}) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = classId != null
          ? await _studentService.getStudentsByClass('Bearer $token', classId)
          : await _studentService.getAllStudents('Bearer $token');

      if (response.isSuccess && response.data != null) {
        state = state.copyWith(
          students: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error:
              response.error ?? response.message ?? 'Failed to load students',
        );
      }
    } catch (e) {
      _logger.e('Error loading students: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> createStudent(Map<String, dynamic> studentData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response =
          await _studentService.createStudent('Bearer $token', studentData);

      if (response.isSuccess && response.data != null) {
        final updatedStudents = [...state.students, response.data!];
        state = state.copyWith(
          students: updatedStudents,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error:
              response.error ?? response.message ?? 'Failed to create student',
        );
      }
    } catch (e) {
      _logger.e('Error creating student: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> updateStudent(int id, Map<String, dynamic> studentData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response =
          await _studentService.updateStudent('Bearer $token', id, studentData);

      if (response.isSuccess && response.data != null) {
        final updatedStudents = state.students.map((student) {
          return student.id == id ? response.data! : student;
        }).toList();

        state = state.copyWith(
          students: updatedStudents,
          selectedStudent: response.data,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error:
              response.error ?? response.message ?? 'Failed to update student',
        );
      }
    } catch (e) {
      _logger.e('Error updating student: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> deleteStudent(int id) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = await _studentService.deleteStudent('Bearer $token', id);

      if (response.isSuccess) {
        final updatedStudents =
            state.students.where((student) => student.id != id).toList();

        state = state.copyWith(
          students: updatedStudents,
          selectedStudent:
              state.selectedStudent?.id == id ? null : state.selectedStudent,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error:
              response.error ?? response.message ?? 'Failed to delete student',
        );
      }
    } catch (e) {
      _logger.e('Error deleting student: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> searchStudents(String query, {int? classId}) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = await _studentService.searchStudents(
        'Bearer $token',
        query,
        classId,
      );

      if (response.isSuccess && response.data != null) {
        state = state.copyWith(
          students: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error:
              response.error ?? response.message ?? 'Failed to search students',
        );
      }
    } catch (e) {
      _logger.e('Error searching students: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  void selectStudent(Student? student) {
    state = state.copyWith(selectedStudent: student);
  }

  void clearError() {
    state = state.copyWith(error: null);
  }

  Future<String> _getToken() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final token = prefs.getString('auth_token');

      if (token != null && token.isNotEmpty) {
        return token;
      }

      // Return empty string if no token found
      return '';
    } catch (e) {
      // Log the error or handle appropriately
      _logger.e('Error retrieving auth token: $e');
      return '';
    }
  }

  Future<void> saveToken(String token) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('auth_token', token);
    } catch (e) {
      _logger.e('Error saving auth token: $e');
    }
  }

  Future<void> clearToken() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.remove('auth_token');
    } catch (e) {
      _logger.e('Error clearing auth token: $e');
    }
  }

  Future<bool> hasValidToken() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final token = prefs.getString('auth_token');
      return token != null && token.isNotEmpty;
    } catch (e) {
      _logger.e('Error checking token validity: $e');
      return false;
    }
  }
}

// Additional providers for convenience
final studentStateProvider = studentProvider;

// Provider for students by class
final studentsByClassProvider =
    Provider.family<List<Student>, int>((ref, classId) {
  final studentState = ref.watch(studentProvider);
  return studentState.students
      .where((student) => student.classId == classId)
      .toList();
});
