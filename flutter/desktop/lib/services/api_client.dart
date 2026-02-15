import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import 'auth_service.dart';
import 'class_service.dart';
import 'student_service.dart';
import 'attendance_service.dart';
import 'api_response.dart';
import 'backup_service.dart';
import 'cache_service.dart';
import 'sync_service.dart';
import 'update_service.dart';

class ApiClient {
  late final Dio _dio;
  final Logger _logger = Logger();

  ApiClient({String? baseUrl}) {
    _dio = Dio(BaseOptions(
      baseUrl: baseUrl ?? 'http://localhost:3000/api',
      connectTimeout: const Duration(seconds: 30),
      receiveTimeout: const Duration(seconds: 30),
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
    ));

    _setupInterceptors();
  }

  Dio get dio => _dio;

  void _setupInterceptors() {
    // Request interceptor
    _dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: (options, handler) {
          _logger.d('REQUEST: ${options.method} ${options.path}');
          if (options.data != null) {
            _logger.d('DATA: ${options.data}');
          }
          handler.next(options);
        },
        onResponse: (response, handler) {
          _logger.d(
              'RESPONSE: ${response.statusCode} ${response.requestOptions.path}');
          handler.next(response);
        },
        onError: (error, handler) {
          _logger.e('ERROR: ${error.message}');
          _logger.e('RESPONSE: ${error.response?.data}');
          handler.next(error);
        },
      ),
    );

    // Error handling interceptor
    _dio.interceptors.add(
      InterceptorsWrapper(
        onError: (error, handler) {
          final response = error.response;
          if (response != null) {
            try {
              final apiResponse = ApiResponse<dynamic>.fromJson(
                response.data,
                (json) => json,
              );
              final dioException = DioException(
                requestOptions: error.requestOptions,
                response: Response(
                  requestOptions: error.requestOptions,
                  data: apiResponse,
                  statusCode: response.statusCode,
                  statusMessage: response.statusMessage,
                  headers: response.headers,
                  extra: response.extra,
                ),
                type: error.type,
                error: apiResponse.error ?? apiResponse.message,
              );
              handler.reject(dioException);
              return;
            } catch (e) {
              // If we can't parse the error, continue with the original error
            }
          }
          handler.next(error);
        },
      ),
    );
  }

  void setAuthToken(String token) {
    _dio.options.headers['Authorization'] = 'Bearer $token';
  }

  void clearAuthToken() {
    _dio.options.headers.remove('Authorization');
  }
}

// Provider for API client
final apiClientProvider = Provider<ApiClient>((ref) {
  return ApiClient();
});

// Provider for AuthService
final authServiceProvider = Provider<AuthService>((ref) {
  final apiClient = ref.watch(apiClientProvider);
  return AuthService(apiClient.dio);
});

// Provider for ClassService
final classServiceProvider = Provider<ClassService>((ref) {
  final apiClient = ref.watch(apiClientProvider);
  return ClassService(apiClient.dio);
});

// Provider for StudentService
final studentServiceProvider = Provider<StudentService>((ref) {
  final apiClient = ref.watch(apiClientProvider);
  return StudentService(apiClient.dio);
});

// Provider for AttendanceService
final attendanceServiceProvider = Provider<AttendanceService>((ref) {
  final apiClient = ref.watch(apiClientProvider);
  return AttendanceService(apiClient.dio);
});

// Provider for BackupService
final backupServiceProvider = Provider<BackupService>((ref) {
  return BackupService();
});

// Provider for CacheService
final cacheServiceProvider = Provider<CacheService>((ref) {
  return CacheService();
});

// Provider for SyncService
final syncServiceProvider = Provider<SyncService>((ref) {
  return SyncService();
});

// Provider for UpdateService
final updateServiceProvider = Provider<UpdateService>((ref) {
  return UpdateService();
});
