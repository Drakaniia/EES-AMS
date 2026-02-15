import 'package:get_it/get_it.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:dio/dio.dart';
import '../../infrastructure/database/attendance_repository_impl.dart';
import '../../infrastructure/database/student_repository_impl.dart';
import '../../infrastructure/database/class_repository_impl.dart';
import '../../services/attendance_service.dart';
import '../../services/student_service.dart';
import '../../services/class_service.dart';
import '../../services/api_client.dart';

/// Global service locator for dependency injection.
///
/// This GetIt instance manages the lifecycle of all services and repositories
/// throughout the application, providing dependency injection capabilities.
final GetIt getIt = GetIt.instance;

/// Configures dependency injection for the application.
///
/// This function should be called once at application startup to register
/// all services, repositories, and other dependencies.
Future<void> configureDependencies() async {
  // Register shared preferences
  final sharedPreferences = await SharedPreferences.getInstance();
  getIt.registerSingleton<SharedPreferences>(sharedPreferences);

  // Register Dio client
  getIt.registerLazySingleton<Dio>(() => ApiClient().dio);

  // Register services
  getIt.registerLazySingleton<AttendanceService>(
      () => AttendanceService(getIt<Dio>()));
  getIt.registerLazySingleton<StudentService>(
      () => StudentService(getIt<Dio>()));
  getIt.registerLazySingleton<ClassService>(() => ClassService(getIt<Dio>()));

  // Register repositories
  getIt.registerLazySingleton<AttendanceRepositoryImpl>(
      () => AttendanceRepositoryImpl(getIt<AttendanceService>()));
  getIt.registerLazySingleton<StudentRepositoryImpl>(
      () => StudentRepositoryImpl(getIt<StudentService>()));
  getIt.registerLazySingleton<ClassRepositoryImpl>(
      () => ClassRepositoryImpl(getIt<ClassService>()));
}
