import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import '../models/attendance.dart';
import '../services/attendance_service.dart';
import '../services/api_client.dart';

class AttendanceState {
  final List<AttendanceRecord> records;
  final AttendanceStats? stats;
  final bool isLoading;
  final String? error;

  const AttendanceState({
    this.records = const [],
    this.stats,
    this.isLoading = false,
    this.error,
  });

  AttendanceState copyWith({
    List<AttendanceRecord>? records,
    AttendanceStats? stats,
    bool? isLoading,
    String? error,
  }) {
    return AttendanceState(
      records: records ?? this.records,
      stats: stats ?? this.stats,
      isLoading: isLoading ?? this.isLoading,
      error: error ?? this.error,
    );
  }
}

final attendanceProvider =
    StateNotifierProvider<AttendanceNotifier, AttendanceState>((ref) {
  final attendanceService = ref.read(attendanceServiceProvider);
  return AttendanceNotifier(attendanceService);
});

class AttendanceNotifier extends StateNotifier<AttendanceState> {
  final AttendanceService _attendanceService;
  final Logger _logger;

  AttendanceNotifier(this._attendanceService, [Logger? logger])
      : _logger = logger ?? Logger(),
        super(const AttendanceState());

  Future<void> loadStats({int? classId, DateTime? date}) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = await _attendanceService.getAttendanceStats(
        'Bearer $token',
        classId: classId,
        date: date?.toIso8601String(),
      );

      if (response.isSuccess && response.data != null) {
        state = state.copyWith(
          stats: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ??
              response.message ??
              'Failed to load attendance stats',
        );
      }
    } catch (e) {
      _logger.e('Error loading attendance stats: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> loadAttendance({int? classId, DateTime? date}) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response = await _attendanceService.getAllAttendance(
        'Bearer $token',
        classId: classId,
        date: date?.toIso8601String(),
      );

      if (response.isSuccess && response.data != null) {
        state = state.copyWith(
          records: response.data!,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ??
              response.message ??
              'Failed to load attendance records',
        );
      }
    } catch (e) {
      _logger.e('Error loading attendance records: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> markAttendance(
      int classId, DateTime date, Map<int, AttendanceStatus> attendance) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final attendanceData = attendance.entries.map((entry) {
        return {
          'student_id': entry.key,
          'class_id': classId,
          'date': date.toIso8601String(),
          'status': entry.value.name,
        };
      }).toList();

      final token = await _getToken();
      final response = await _attendanceService.createBulkAttendance(
        'Bearer $token',
        attendanceData,
      );

      if (response.isSuccess && response.data != null) {
        final updatedRecords = [...state.records, ...response.data!];
        state = state.copyWith(
          records: updatedRecords,
          isLoading: false,
          error: null,
        );

        // Refresh stats after marking attendance
        await loadStats(classId: classId, date: date);
      } else {
        state = state.copyWith(
          isLoading: false,
          error:
              response.error ?? response.message ?? 'Failed to mark attendance',
        );
      }
    } catch (e) {
      _logger.e('Error marking attendance: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> updateAttendanceRecord(int id, AttendanceStatus status,
      {String? notes}) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final attendanceData = {
        'status': status.name,
        if (notes != null) 'notes': notes,
      };

      final token = await _getToken();
      final response = await _attendanceService.updateAttendance(
        'Bearer $token',
        id,
        attendanceData,
      );

      if (response.isSuccess && response.data != null) {
        final updatedRecords = state.records.map((record) {
          return record.id == id ? response.data! : record;
        }).toList();

        state = state.copyWith(
          records: updatedRecords,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ??
              response.message ??
              'Failed to update attendance',
        );
      }
    } catch (e) {
      _logger.e('Error updating attendance: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
  }

  Future<void> deleteAttendanceRecord(int id) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      final token = await _getToken();
      final response =
          await _attendanceService.deleteAttendance('Bearer $token', id);

      if (response.isSuccess) {
        final updatedRecords =
            state.records.where((record) => record.id != id).toList();

        state = state.copyWith(
          records: updatedRecords,
          isLoading: false,
          error: null,
        );
      } else {
        state = state.copyWith(
          isLoading: false,
          error: response.error ??
              response.message ??
              'Failed to delete attendance',
        );
      }
    } catch (e) {
      _logger.e('Error deleting attendance: $e');
      state = state.copyWith(
        isLoading: false,
        error: 'An unexpected error occurred',
      );
    }
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
