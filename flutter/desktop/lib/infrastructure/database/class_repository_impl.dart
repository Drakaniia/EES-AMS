import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import '../../services/class_service.dart';
import '../../domain/entities/class_entity.dart';
import '../../domain/core/result.dart';
import '../../domain/exceptions/domain_exceptions.dart';

/// Concrete implementation of the ClassRepository interface.
class ClassRepositoryImpl {
  final ClassService _classService;
  final _secureStorage = const FlutterSecureStorage();

  ClassRepositoryImpl(this._classService);

  Future<String> _getAuthToken() async {
    try {
      // Try to get token from secure storage
      final token = await _secureStorage.read(key: 'auth_token');

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
      await _secureStorage.write(key: 'auth_token', value: token);
    } catch (e) {
      // Log the error or handle appropriately
      // print('Error saving auth token: $e');
      rethrow;
    }
  }

  Future<void> clearAuthToken() async {
    try {
      await _secureStorage.delete(key: 'auth_token');
    } catch (e) {
      // Log the error or handle appropriately
      // print('Error clearing auth token: $e');
    }
  }

  Future<Result<List<ClassEntity>>> getAllClasses() async {
    try {
      final authToken = await _getAuthToken();
      final response = await _classService.getAllClasses(authToken);

      if (response.success && response.data != null) {
        final classes = response.data!
            .map((classItem) => ClassEntity.fromModel(classItem))
            .toList();
        return Result.success(classes);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to fetch classes'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<ClassEntity>> getClassById(int id) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _classService.getClassById(authToken, id);

      if (response.success && response.data != null) {
        final classItem = ClassEntity.fromModel(response.data!);
        return Result.success(classItem);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Class not found'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<ClassEntity>> createClass(ClassEntity classEntity) async {
    try {
      final classData = classEntity.toModel().toJson();
      final authToken = await _getAuthToken();
      final response = await _classService.createClass(authToken, classData);

      if (response.success && response.data != null) {
        final createdClass = ClassEntity.fromModel(response.data!);
        return Result.success(createdClass);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to create class'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<ClassEntity>> updateClass(ClassEntity classEntity) async {
    try {
      if (classEntity.id == null) {
        return Result.failure(DataException('Class ID is required for update'));
      }

      final classData = classEntity.toModel().toJson();
      final authToken = await _getAuthToken();
      final response = await _classService.updateClass(
          authToken, classEntity.id!, classData);

      if (response.success && response.data != null) {
        final updatedClass = ClassEntity.fromModel(response.data!);
        return Result.success(updatedClass);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to update class'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<void>> deleteClass(int id) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _classService.deleteClass(authToken, id);

      if (response.success) {
        return Result.success(null);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to delete class'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<List<ClassEntity>>> searchClasses(String query) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _classService.searchClasses(authToken, query);

      if (response.success && response.data != null) {
        final classes = response.data!
            .map((classItem) => ClassEntity.fromModel(classItem))
            .toList();
        return Result.success(classes);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Failed to search classes'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }
}
