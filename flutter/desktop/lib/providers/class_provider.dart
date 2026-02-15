import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import '../models/class.dart';
import '../services/class_service.dart';
import '../services/api_client.dart';

class ClassState {
  final List<Class> classes;
  final Class? selectedClass;
  final bool isLoading;
  final String? error;

  const ClassState({
    this.classes = const [],
    this.selectedClass,
    this.isLoading = false,
    this.error,
  });

  ClassState copyWith({
    List<Class>? classes,
    Class? selectedClass,
    bool? isLoading,
    String? error,
  }) {
    return ClassState(
      classes: classes ?? this.classes,
      selectedClass: selectedClass ?? this.selectedClass,
      isLoading: isLoading ?? this.isLoading,
      error: error ?? this.error,
    );
  }
}

final classProvider = StateNotifierProvider<ClassNotifier, ClassState>((ref) {
  final classService = ref.read(classServiceProvider);
  return ClassNotifier(classService);
});

class ClassNotifier extends StateNotifier<ClassState> {
  final ClassService _classService;
  final Logger _logger;

  ClassNotifier(this._classService, [Logger? logger])
      : _logger = logger ?? Logger(),
        super(const ClassState());

  Future<void> loadClasses() async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = await _classService.getAllClasses('Bearer $token');

      if (response.isSuccess && response.data != null) {
        state = state.copyWith(
          classes: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Failed to load classes',
        );
      }
    } catch (e) {
      _logger.e('Error loading classes: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> createClass(Map<String, dynamic> classData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response =
          await _classService.createClass('Bearer $token', classData);

      if (response.isSuccess && response.data != null) {
        final updatedClasses = [...state.classes, response.data!];
        state = state.copyWith(
          classes: updatedClasses,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Failed to create class',
        );
      }
    } catch (e) {
      _logger.e('Error creating class: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> updateClass(int id, Map<String, dynamic> classData) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response =
          await _classService.updateClass('Bearer $token', id, classData);

      if (response.isSuccess && response.data != null) {
        final updatedClasses = state.classes.map((cls) {
          return cls.id == id ? response.data! : cls;
        }).toList();

        state = state.copyWith(
          classes: updatedClasses,
          selectedClass: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Failed to update class',
        );
      }
    } catch (e) {
      _logger.e('Error updating class: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> deleteClass(int id) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = await _classService.deleteClass('Bearer $token', id);

      if (response.isSuccess) {
        final updatedClasses =
            state.classes.where((cls) => cls.id != id).toList();

        state = state.copyWith(
          classes: updatedClasses,
          selectedClass:
              state.selectedClass?.id == id ? null : state.selectedClass,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ?? response.message ?? 'Failed to delete class',
        );
      }
    } catch (e) {
      _logger.e('Error deleting class: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  void selectClass(Class? classItem) {
    state = state.copyWith(selectedClass: classItem);
  }

  void clearError() {
    state = state.copyWith(error: null);
  }

  Future<String> _getToken() async {
    try {
      // Try to get token from secure storage
      const secureStorage = FlutterSecureStorage();
      final token = await secureStorage.read(key: 'auth_token');

      if (token != null && token.isNotEmpty) {
        return token;
      }

      // Return empty string if no token found
      return '';
    } catch (e) {
      _logger.e('Error retrieving auth token: $e');
      return '';
    }
  }

  Future<void> saveToken(String token) async {
    try {
      const secureStorage = FlutterSecureStorage();
      await secureStorage.write(key: 'auth_token', value: token);
    } catch (e) {
      _logger.e('Error saving auth token: $e');
    }
  }

  Future<void> clearToken() async {
    try {
      const secureStorage = FlutterSecureStorage();
      await secureStorage.delete(key: 'auth_token');
    } catch (e) {
      _logger.e('Error clearing auth token: $e');
    }
  }

  Future<bool> hasValidToken() async {
    try {
      const secureStorage = FlutterSecureStorage();
      final token = await secureStorage.read(key: 'auth_token');
      return token != null && token.isNotEmpty;
    } catch (e) {
      _logger.e('Error checking token validity: $e');
      return false;
    }
  }
}
