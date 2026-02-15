import 'package:dio/dio.dart';
import '../models/user.dart';
import 'api_response.dart';

class AuthService {
  final Dio _dio;
  final String? _baseUrl;

  AuthService(this._dio, {String? baseUrl}) : _baseUrl = baseUrl;

  Future<ApiResponse<AuthResponse>> login(AuthCredentials credentials) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/auth/login',
        data: credentials.toJson(),
      );
      return ApiResponse<AuthResponse>.fromJson(
        response.data,
        (json) => AuthResponse.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AuthResponse>> register(Map<String, dynamic> userData) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/auth/register',
        data: userData,
      );
      return ApiResponse<AuthResponse>.fromJson(
        response.data,
        (json) => AuthResponse.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AuthResponse>> refreshToken(String refreshToken) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/auth/refresh',
        data: {'refreshToken': refreshToken},
      );
      return ApiResponse<AuthResponse>.fromJson(
        response.data,
        (json) => AuthResponse.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<void>> logout(String token) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/auth/logout',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<void>.fromJson(response.data, (_) {});
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<User>> getCurrentUser(String token) async {
    try {
      final response = await _dio.get(
        '$_baseUrl/auth/me',
        options: Options(headers: {'Authorization': token}),
      );
      return ApiResponse<User>.fromJson(
        response.data,
        (json) => User.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AuthResponse>> signInWithGoogle(Map<String, dynamic> googleData) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/auth/google',
        data: googleData,
      );
      return ApiResponse<AuthResponse>.fromJson(
        response.data,
        (json) => AuthResponse.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<User>> updateProfile(
    String token,
    Map<String, dynamic> profileData,
  ) async {
    try {
      final response = await _dio.put(
        '$_baseUrl/auth/profile',
        options: Options(headers: {'Authorization': token}),
        data: profileData,
      );
      return ApiResponse<User>.fromJson(
        response.data,
        (json) => User.fromJson(json as Map<String, dynamic>),
      );
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<void>> changePassword(
    String token,
    Map<String, dynamic> passwordData,
  ) async {
    try {
      final response = await _dio.post(
        '$_baseUrl/auth/change-password',
        options: Options(headers: {'Authorization': token}),
        data: passwordData,
      );
      return ApiResponse<void>.fromJson(response.data, (_) {});
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