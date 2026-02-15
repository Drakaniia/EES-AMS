import 'package:dio/dio.dart';
import '../models/class.dart';
import 'api_response.dart';

class ClassService {
  final Dio _dio;
  final String? _baseUrl;

  ClassService(this._dio, {String? baseUrl}) : _baseUrl = baseUrl;

  Future<ApiResponse<List<Class>>> getAllClasses(String token) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/classes',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<List<Class>>.fromJson(
        response.data,
        (json) => (json as List)
            .map((item) => Class.fromJson(item as Map<String, dynamic>))
            .toList(),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Class>> getClassById(String token, int id) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/classes/$id',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<Class>.fromJson(
        response.data,
        (json) => Class.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Class>> createClass(
    String token,
    Map<String, dynamic> classData,
  ) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/classes',
        options: Options(headers: {'Authorization': token}),
        data: classData,
      );
      return ApiResponse<Class>.fromJson(
        response.data,
        (json) => Class.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Class>> updateClass(
    String token,
    int id,
    Map<String, dynamic> classData,
  ) async {
    try {
      final response = await _dio.put(
        '$_baseUrl/classes/$id',
        options: Options(headers: {'Authorization': token}),
        data: classData,
      );
      return ApiResponse<Class>.fromJson(
        response.data,
        (json) => Class.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<void>> deleteClass(String token, int id) async {
    try {
      final response = await _dio.delete(
        '$_baseUrl/classes/$id',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<void>.fromJson(response.data, (_) {});
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<dynamic>>> getClassStudents(String token, int id) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/classes/$id/students',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<List<dynamic>>.fromJson(
        response.data,
        (json) => json as List,
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<Map<String, dynamic>>> getClassStats(
    String token,
    int id,
  ) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/classes/$id/stats',
        options: Options(headers: {'Authorization': token}),
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

  Future<ApiResponse<List<Class>>> searchClasses(
    String token,
    String query,
  ) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/classes/search',
        options: Options(headers: {'Authorization': token}),
        queryParameters: {'q': query},
      );
      return ApiResponse<List<Class>>.fromJson(
        response.data,
        (json) => (json as List)
            .map((item) => Class.fromJson(item as Map<String, dynamic>))
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
