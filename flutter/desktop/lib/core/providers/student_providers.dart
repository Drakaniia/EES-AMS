import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:dio/dio.dart';
import '../../services/student_service.dart';
import '../../services/config_service.dart';
import '../../services/secure_storage_service.dart';
import '../../infrastructure/database/student_repository_impl.dart';

// Provider for ConfigService
final configServiceProvider = Provider<ConfigService>((ref) {
  return ConfigService();
});

// Provider for API client (Dio)
final apiClientProvider = Provider<Dio>((ref) {
  final dio = Dio();
  final configService = ref.watch(configServiceProvider);
  final secureStorage = SecureStorageService();

  // Set base URL from config (will be loaded asynchronously)
  configService.getBaseUrl().then((baseUrl) {
    dio.options.baseUrl = baseUrl;
  });

  dio.options.connectTimeout = const Duration(seconds: 30);
  dio.options.receiveTimeout = const Duration(seconds: 30);

  // Add authentication interceptor
  dio.interceptors.add(InterceptorsWrapper(
    onRequest: (options, handler) async {
      // Add auth token from secure storage
      final token = await secureStorage.getAccessToken();
      if (token != null && token.isNotEmpty) {
        options.headers['Authorization'] = 'Bearer $token';
      }
      handler.next(options);
    },
    onError: (error, handler) async {
      // Handle 401 Unauthorized - token expired
      if (error.response?.statusCode == 401) {
        // Could implement token refresh here
      }
      handler.next(error);
    },
  ));

  return dio;
});

// Provider for StudentService
final studentServiceProvider = Provider<StudentService>((ref) {
  final dio = ref.watch(apiClientProvider);
  return StudentService(dio);
});

// Provider for StudentRepository
final studentRepositoryProvider = Provider<StudentRepositoryImpl>((ref) {
  final service = ref.watch(studentServiceProvider);
  return StudentRepositoryImpl(service);
});
