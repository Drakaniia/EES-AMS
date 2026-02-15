import 'dart:convert';
import 'package:dio/dio.dart';
import '../models/attendance.dart';
import 'api_response.dart';

class AttendanceService {
  final Dio _dio;
  
  AttendanceService(this._dio);

  Future<ApiResponse<List<AttendanceRecord>>> getAllAttendance(
    String token, {
    int? classId,
    String? date,
    String? startDate,
    String? endDate,
  }) async {
    try {
      final response = await _dio.get('/attendance', 
        options: Options(headers: {'Authorization': token}),
        queryParameters: {
          if (classId != null) 'classId': classId,
          if (date != null) 'date': date,
          if (startDate != null) 'startDate': startDate,
          if (endDate != null) 'endDate': endDate,
        },
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      } else {
        return ApiResponse.fromJson(response.data, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AttendanceRecord>> getAttendanceById(String token, int id) async {
    try {
      final response = await _dio.get('/attendance/$id',
        options: Options(headers: {'Authorization': token}),
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => AttendanceRecord.fromJson(json as Map<String, dynamic>));
      } else {
        return ApiResponse.fromJson(response.data, (json) => AttendanceRecord.fromJson(json as Map<String, dynamic>));
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AttendanceRecord>> createAttendance(
    String token,
    Map<String, dynamic> attendanceData,
  ) async {
    try {
      final response = await _dio.post('/attendance',
        options: Options(headers: {'Authorization': token}),
        data: attendanceData,
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => AttendanceRecord.fromJson(json as Map<String, dynamic>));
      } else {
        return ApiResponse.fromJson(response.data, (json) => AttendanceRecord.fromJson(json as Map<String, dynamic>));
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AttendanceRecord>> updateAttendance(
    String token,
    int id,
    Map<String, dynamic> attendanceData,
  ) async {
    try {
      final response = await _dio.put('/attendance/$id',
        options: Options(headers: {'Authorization': token}),
        data: attendanceData,
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => AttendanceRecord.fromJson(json as Map<String, dynamic>));
      } else {
        return ApiResponse.fromJson(response.data, (json) => AttendanceRecord.fromJson(json as Map<String, dynamic>));
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<void>> deleteAttendance(String token, int id) async {
    try {
      final response = await _dio.delete('/attendance/$id',
        options: Options(headers: {'Authorization': token}),
      );

      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (_) {});
      } else {
        return ApiResponse.fromJson(response.data, (_) {});
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<AttendanceRecord>>> createBulkAttendance(
    String token,
    List<Map<String, dynamic>> attendanceData,
  ) async {
    try {
      final response = await _dio.post('/attendance/bulk',
        options: Options(headers: {'Authorization': token}),
        data: attendanceData,
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      } else {
        return ApiResponse.fromJson(response.data, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<AttendanceStats>> getAttendanceStats(
    String token, {
    int? classId,
    String? date,
    String? startDate,
    String? endDate,
  }) async {
    try {
      final response = await _dio.get('/attendance/stats',
        options: Options(headers: {'Authorization': token}),
        queryParameters: {
          if (classId != null) 'classId': classId,
          if (date != null) 'date': date,
          if (startDate != null) 'startDate': startDate,
          if (endDate != null) 'endDate': endDate,
        },
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => AttendanceStats.fromJson(json as Map<String, dynamic>));
      } else {
        return ApiResponse.fromJson(response.data, (json) => AttendanceStats.fromJson(json as Map<String, dynamic>));
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<AttendanceRecord>>> getAttendanceByClassAndDate(
    String token,
    int classId,
    String date,
  ) async {
    try {
      final response = await _dio.get('/attendance/class/$classId/date/$date',
        options: Options(headers: {'Authorization': token}),
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      } else {
        return ApiResponse.fromJson(response.data, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Future<ApiResponse<List<AttendanceRecord>>> getStudentAttendanceHistory(
    String token,
    int studentId, {
    String? startDate,
    String? endDate,
  }) async {
    try {
      final response = await _dio.get('/attendance/student/$studentId',
        options: Options(headers: {'Authorization': token}),
        queryParameters: {
          if (startDate != null) 'startDate': startDate,
          if (endDate != null) 'endDate': endDate,
        },
      );
      
      if (response.data is String) {
        final jsonData = jsonDecode(response.data);
        return ApiResponse.fromJson(jsonData, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      } else {
        return ApiResponse.fromJson(response.data, (json) => 
          (json as List).map((item) => AttendanceRecord.fromJson(item)).toList());
      }
    } catch (e) {
      throw _handleError(e);
    }
  }

  Exception _handleError(dynamic error) {
    if (error is DioException) {
      switch (error.type) {
        case DioExceptionType.connectionTimeout:
        case DioExceptionType.sendTimeout:
        case DioExceptionType.receiveTimeout:
          return Exception('Connection timeout');
        case DioExceptionType.badResponse:
          final statusCode = error.response?.statusCode;
          if (statusCode != null) {
            switch (statusCode) {
              case 400:
                return Exception('Bad request');
              case 401:
                return Exception('Unauthorized');
              case 403:
                return Exception('Forbidden');
              case 404:
                return Exception('Not found');
              case 500:
                return Exception('Internal server error');
            }
          }
          return Exception('HTTP Error: $statusCode');
        case DioExceptionType.cancel:
          return Exception('Request cancelled');
        case DioExceptionType.unknown:
          return Exception('Network error');
        default:
          return Exception('Unknown error occurred');
      }
    }
    return Exception('Unknown error: $error');
  }
}