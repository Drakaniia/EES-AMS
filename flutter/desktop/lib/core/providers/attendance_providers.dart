import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../services/attendance_service.dart';
import '../../infrastructure/database/attendance_repository_impl.dart';
import 'student_providers.dart';

// Provider for AttendanceService
final attendanceServiceProvider = Provider<AttendanceService>((ref) {
  final dio = ref.watch(apiClientProvider);
  return AttendanceService(dio);
});

// Provider for AttendanceRepository
final attendanceRepositoryProvider = Provider<AttendanceRepositoryImpl>((ref) {
  final service = ref.watch(attendanceServiceProvider);
  return AttendanceRepositoryImpl(service);
});
