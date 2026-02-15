import '../entities/attendance_entity.dart';
import '../core/result.dart';
import '../exceptions/domain_exceptions.dart';

/// Repository interface for attendance-related data operations.
/// 
/// This interface defines the contract for attendance data access without
/// specifying the implementation details, following the Repository pattern.
abstract class AttendanceRepository {
  /// Records attendance for students.
  /// 
  /// [attendanceRecords] is a list of attendance records to save.
  /// Returns the list of saved [AttendanceRecordEntity] on success.
  /// Throws [ValidationException] on invalid data or [DataException] on save failure.
  Future<Result<List<AttendanceRecordEntity>>> recordAttendance(
    List<AttendanceRecordEntity> attendanceRecords,
  );

  /// Retrieves attendance records for a specific class and date.
  /// 
  /// Returns a list of [AttendanceRecordEntity] for the specified class and date.
  Future<Result<List<AttendanceRecordEntity>>> getAttendanceByClassAndDate({
    required int classId,
    required DateTime date,
  });

  /// Retrieves attendance records for a specific student within a date range.
  /// 
  /// Returns a list of [AttendanceRecordEntity] for the specified student.
  Future<Result<List<AttendanceRecordEntity>>> getAttendanceByStudentAndDateRange({
    required int studentId,
    required DateTime startDate,
    required DateTime endDate,
  });

  /// Retrieves attendance records for a class within a date range.
  /// 
  /// Returns a list of [AttendanceRecordEntity] for the specified class.
  Future<Result<List<AttendanceRecordEntity>>> getAttendanceByClassAndDateRange({
    required int classId,
    required DateTime startDate,
    required DateTime endDate,
  });

  /// Updates an existing attendance record.
  /// 
  /// Returns updated [AttendanceRecordEntity] on success.
  /// Throws [ValidationException] on invalid data or [DataException] on update failure.
  Future<Result<AttendanceRecordEntity>> updateAttendanceRecord(
    AttendanceRecordEntity record,
  );

  /// Deletes an attendance record.
  /// 
  /// Returns success result on deletion,
  /// throws [DataException] if record not found or deletion fails.
  Future<Result<void>> deleteAttendanceRecord(int id);

  /// Retrieves attendance statistics for a class on a specific date.
  /// 
  /// Returns [AttendanceStatsEntity] for the specified class and date.
  Future<Result<AttendanceStatsEntity>> getAttendanceStatsForClass({
    required int classId,
    required DateTime date,
  });

  /// Retrieves attendance statistics for a student within a date range.
  /// 
  /// Returns [AttendanceStatsEntity] for the specified student.
  Future<Result<AttendanceStatsEntity>> getAttendanceStatsForStudent({
    required int studentId,
    required DateTime startDate,
    required DateTime endDate,
  });

  /// Retrieves attendance statistics for all classes on a specific date.
  /// 
  /// Returns a map of class IDs to their [AttendanceStatsEntity].
  Future<Result<Map<int, AttendanceStatsEntity>>> getAttendanceStatsForAllClasses({
    required DateTime date,
  });

  /// Exports attendance records for a class to Excel format.
  /// 
  /// [startDate] and [endDate] define the date range.
  /// Returns Excel data as a list of maps.
  Future<Result<List<Map<String, dynamic>>>> exportAttendanceToExcel({
    required int classId,
    required DateTime startDate,
    required DateTime endDate,
  });
}