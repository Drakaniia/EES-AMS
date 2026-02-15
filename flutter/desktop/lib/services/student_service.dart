import 'package:dio/dio.dart';
import '../models/student.dart';
import 'api_response.dart';

class StudentService {
  final Dio _dio;
  final String? _baseUrl;

  StudentService(this._dio, {String? baseUrl}) : _baseUrl = baseUrl;

  Future<ApiResponse<List<Student>>> getAllStudents(String token) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/students',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<List<Student>>.fromJson(
        response.data,
        (json) => (json as List)
            .map((item) => Student.fromJson(item))
            .toList(),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Student>> getStudentById(String token, int id) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/students/$id',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<Student>.fromJson(
        response.data,
        (json) => Student.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Student>> createStudent(
    String token,
    Map<String, dynamic> studentData,
  ) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/students',
        options: Options(headers: {'Authorization': token}),
        data: studentData,
      );
      return ApiResponse<Student>.fromJson(
        response.data,
        (json) => Student.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Student>> updateStudent(
    String token,
    int id,
    Map<String, dynamic> studentData,
  ) async {
    try {
      final response = await _dio.put(
        '$_baseUrl/students/$id',
        options: Options(headers: {'Authorization': token}),
        data: studentData,
      );
      return ApiResponse<Student>.fromJson(
        response.data,
        (json) => Student.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<void>> deleteStudent(String token, int id) async {
    try {
      final response = await _dio.delete(
        '$_baseUrl/students/$id',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<void>.fromJson(response.data, (_) {});
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<Student>>> getStudentsByClass(
    String token,
    int classId,
  ) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/students/class/$classId',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<List<Student>>.fromJson(
        response.data,
        (json) => (json as List)
            .map((item) => Student.fromJson(item))
            .toList(),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Map<String, dynamic>>> importStudentsFromExcel(
    String token,
    String filePath,
  ) async {
    try {
      final formData = FormData.fromMap({
        'file': await MultipartFile.fromFile(filePath),
      });
      final response = await _dio.post(
        '$_baseUrl/students/import',
        options: Options(headers: {'Authorization': token}),
        data: formData,
      );
      return ApiResponse<Map<String, dynamic>>.fromJson(
        response.data,
        (json) => json is Map<String, dynamic> 
            ? json 
            : <String, dynamic>{},
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<Student>>> searchStudents(
    String token,
    String query,
    int? classId,
  ) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/students/search',
        options: Options(headers: {'Authorization': token}),
        queryParameters: {
          'q': query,
          if (classId != null) 'classId': classId,
        },
      );
      return ApiResponse<List<Student>>.fromJson(
        response.data,
        (json) => (json as List)
            .map((item) => Student.fromJson(item))
            .toList(),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<Student>>> createBulkStudents(
    String token,
    List<Map<String, dynamic>> studentsData,
  ) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/students/bulk',
        options: Options(headers: {'Authorization': token}),
        data: studentsData,
      );
      return ApiResponse<List<Student>>.fromJson(
        response.data,
        (json) => (json as List)
            .map((item) => Student.fromJson(item))
            .toList(),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  dynamic _handleError(dynamic error) {
    if (error is DioException) {
      if (error.response?.statusCode == 401) {
        throw Exception('Unauthorized: Please check your credentials');
      } else if (error.response?.statusCode == 403) {
        throw Exception('Forbidden: You do not have permission to access this resource');
      } else if (error.response?.statusCode == 404) {
        throw Exception('Not Found: The requested resource was not found');
      } else if (error.response?.statusCode == 500) {
        throw Exception('Internal Server Error: Please try again later');
      }
      throw Exception(error.response?.data?['message'] ?? error.message ?? 'An error occurred');
    }
    throw Exception('Network error: Please check your connection');
  }
}