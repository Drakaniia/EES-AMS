import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import '../../services/attendance_service.dart';
import '../../domain/entities/attendance_entity.dart';
import '../../domain/core/result.dart';
import '../../domain/exceptions/domain_exceptions.dart';

/// Concrete implementation of the AttendanceRepository interface.
class AttendanceRepositoryImpl {
  final AttendanceService _attendanceService;
  final _secureStorage = const FlutterSecureStorage();

  AttendanceRepositoryImpl(this._attendanceService);

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
      // print('Error retrieving auth token: $e'); // Disabled for production safety
      return '';
    }
  }

  Future<void> saveAuthToken(String token) async {
    try {
      await _secureStorage.write(key: 'auth_token', value: token);
    } catch (e) {
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

  Future<Result<List<AttendanceRecordEntity>>> getAllAttendance({
    int? classId,
    DateTime? date,
    DateTime? startDate,
    DateTime? endDate,
  }) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _attendanceService.getAllAttendance(
        authToken,
        classId: classId,
        date: date?.toIso8601String(),
        startDate: startDate?.toIso8601String(),
        endDate: endDate?.toIso8601String(),
      );

      if (response.success && response.data != null) {
        final records = response.data!
            .map((record) => AttendanceRecordEntity.fromModel(record))
            .toList();
        return Result.success(records);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to fetch attendance records'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<AttendanceRecordEntity>> getAttendanceById(int id) async {
    try {
      final authToken = await _getAuthToken();
      final response =
          await _attendanceService.getAttendanceById(authToken, id);

      if (response.success && response.data != null) {
        final record = AttendanceRecordEntity.fromModel(response.data!);
        return Result.success(record);
      } else {
        return Result.failure(
            DataException(response.message ?? 'Attendance record not found'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<AttendanceRecordEntity>> createAttendance(
      AttendanceRecordEntity attendance) async {
    try {
      final attendanceData = attendance.toModel().toJson();
      final authToken = await _getAuthToken();
      final response =
          await _attendanceService.createAttendance(authToken, attendanceData);

      if (response.success && response.data != null) {
        final createdRecord = AttendanceRecordEntity.fromModel(response.data!);
        return Result.success(createdRecord);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to create attendance record'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<AttendanceRecordEntity>> updateAttendance(
      AttendanceRecordEntity attendance) async {
    try {
      if (attendance.id == null) {
        return Result.failure(
            DataException('Attendance ID is required for update'));
      }

      final attendanceData = attendance.toModel().toJson();
      final authToken = await _getAuthToken();
      final response = await _attendanceService.updateAttendance(
          authToken, attendance.id!, attendanceData);

      if (response.success && response.data != null) {
        final updatedRecord = AttendanceRecordEntity.fromModel(response.data!);
        return Result.success(updatedRecord);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to update attendance record'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<void>> deleteAttendance(int id) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _attendanceService.deleteAttendance(authToken, id);

      if (response.success) {
        return Result.success(null);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to delete attendance record'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<List<AttendanceRecordEntity>>> createBulkAttendance(
      List<AttendanceRecordEntity> attendanceRecords) async {
    try {
      final attendanceData =
          attendanceRecords.map((record) => record.toModel().toJson()).toList();
      final authToken = await _getAuthToken();
      final response = await _attendanceService.createBulkAttendance(
          authToken, attendanceData);

      if (response.success && response.data != null) {
        final records = response.data!
            .map((record) => AttendanceRecordEntity.fromModel(record))
            .toList();
        return Result.success(records);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to create bulk attendance records'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<AttendanceStatsEntity>> getAttendanceStats({
    int? classId,
    DateTime? date,
    DateTime? startDate,
    DateTime? endDate,
  }) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _attendanceService.getAttendanceStats(
        authToken,
        classId: classId,
        date: date?.toIso8601String(),
        startDate: startDate?.toIso8601String(),
        endDate: endDate?.toIso8601String(),
      );

      if (response.success && response.data != null) {
        final stats = AttendanceStatsEntity.fromModel(response.data!);
        return Result.success(stats);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to fetch attendance statistics'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<List<AttendanceRecordEntity>>> getAttendanceByClassAndDate(
      int classId, DateTime date) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _attendanceService.getAttendanceByClassAndDate(
          authToken, classId, date.toIso8601String());

      if (response.success && response.data != null) {
        final records = response.data!
            .map((record) => AttendanceRecordEntity.fromModel(record))
            .toList();
        return Result.success(records);
      } else {
        return Result.failure(DataException(response.message ??
            'Failed to fetch attendance for class and date'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }

  Future<Result<List<AttendanceRecordEntity>>> getStudentAttendanceHistory(
    int studentId, {
    DateTime? startDate,
    DateTime? endDate,
  }) async {
    try {
      final authToken = await _getAuthToken();
      final response = await _attendanceService.getStudentAttendanceHistory(
        authToken,
        studentId,
        startDate: startDate?.toIso8601String(),
        endDate: endDate?.toIso8601String(),
      );

      if (response.success && response.data != null) {
        final records = response.data!
            .map((record) => AttendanceRecordEntity.fromModel(record))
            .toList();
        return Result.success(records);
      } else {
        return Result.failure(DataException(
            response.message ?? 'Failed to fetch student attendance history'));
      }
    } catch (e) {
      return Result.failure(DataException('Network error: ${e.toString()}'));
    }
  }
}
