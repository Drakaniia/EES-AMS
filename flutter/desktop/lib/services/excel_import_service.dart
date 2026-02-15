import 'dart:io';
import 'package:excel/excel.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import '../models/student.dart';
import '../providers/student_provider.dart';
import '../providers/class_provider.dart';

class ExcelImportService {
  final Logger _logger = Logger();

  Future<File?> pickExcelFile() async {
    try {
      FilePickerResult? result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['xlsx', 'xls'],
        allowMultiple: false,
      );

      if (result != null && result.files.single.path != null) {
        return File(result.files.single.path!);
      }
      return null;
    } catch (e) {
      _logger.e('Error picking Excel file: $e');
      return null;
    }
  }

  Future<List<Student>> parseStudentsFromExcel(File file,
      {int? classId}) async {
    try {
      final bytes = await file.readAsBytes();
      final excel = Excel.decodeBytes(bytes);

      final students = <Student>[];

      for (final table in excel.tables.keys) {
        final sheet = excel.tables[table]!;

        // Skip if sheet is empty
        if (sheet.rows.isEmpty) continue;

        // Assume first row contains headers
        final headers = _extractHeaders(sheet.rows[0]);

        // Validate required headers
        if (!_hasRequiredHeaders(headers)) {
          throw ExcelImportException(
            'Excel file must contain headers: Student ID, First Name, Last Name',
          );
        }

        // Process data rows
        for (int i = 1; i < sheet.rows.length; i++) {
          final row = sheet.rows[i];

          try {
            final student = _parseStudentFromRow(row, headers, classId);
            if (student != null) {
              students.add(student);
            }
          } catch (e) {
            _logger.w('Error parsing row $i: $e');
            // Continue with next row
          }
        }

        // Only process first sheet for now
        break;
      }

      return students;
    } catch (e) {
      _logger.e('Error parsing Excel file: $e');
      throw ExcelImportException('Failed to parse Excel file: $e');
    }
  }

  List<String> _extractHeaders(List<Data?> row) {
    return row.map((cell) {
      if (cell?.value != null) {
        return cell!.value.toString().trim().toLowerCase();
      }
      return '';
    }).toList();
  }

  bool _hasRequiredHeaders(List<String> headers) {
    final requiredHeaders = [
      'student id',
      'student_id',
      'first name',
      'first_name',
      'last name',
      'last_name'
    ];

    for (final required in requiredHeaders) {
      if (!headers.any((header) => header.contains(required))) {
        return false;
      }
    }

    return true;
  }

  Student? _parseStudentFromRow(
      List<Data?> row, List<String> headers, int? classId) {
    final studentId = _getCellValue(row, headers, ['student id', 'student_id']);
    final firstName = _getCellValue(row, headers, ['first name', 'first_name']);
    final lastName = _getCellValue(row, headers, ['last name', 'last_name']);
    final email = _getCellValue(row, headers, ['email']);
    final phone =
        _getCellValue(row, headers, ['phone', 'phone number', 'phone_number']);

    // Skip empty rows or rows with missing required fields
    if (studentId == null || firstName == null || lastName == null) {
      return null;
    }

    return Student(
      studentId: studentId,
      firstName: firstName,
      lastName: lastName,
      email: email,
      phone: phone,
      classId: classId,
    );
  }

  String? _getCellValue(
      List<Data?> row, List<String> headers, List<String> searchTerms) {
    for (int i = 0; i < headers.length && i < row.length; i++) {
      final header = headers[i];
      for (final term in searchTerms) {
        if (header.contains(term)) {
          final cell = row[i];
          if (cell?.value != null) {
            return cell!.value.toString().trim();
          }
        }
      }
    }
    return null;
  }

  Future<ExcelImportResult> importStudents(
    List<Student> students,
    StudentNotifier studentNotifier,
    ClassNotifier classNotifier,
  ) async {
    try {
      int successCount = 0;
      int errorCount = 0;
      final errors = <String>[];

      // Process students in batches to avoid overwhelming the API
      const batchSize = 10;

      for (int i = 0; i < students.length; i += batchSize) {
        final batch = students.skip(i).take(batchSize).toList();

        for (final student in batch) {
          try {
            await studentNotifier.createStudent({
              'student_id': student.studentId,
              'first_name': student.firstName,
              'last_name': student.lastName,
              if (student.email != null) 'email': student.email!,
              if (student.phone != null) 'phone': student.phone!,
              if (student.classId != null) 'class_id': student.classId!,
            });

            successCount++;
          } catch (e) {
            errorCount++;
            errors.add('Failed to import ${student.displayName}: $e');
          }
        }

        // Small delay between batches
        if (i + batchSize < students.length) {
          await Future.delayed(const Duration(milliseconds: 100));
        }
      }

      return ExcelImportResult(
        successCount: successCount,
        errorCount: errorCount,
        errors: errors,
        totalProcessed: students.length,
      );
    } catch (e) {
      throw ExcelImportException('Import failed: $e');
    }
  }

  Future<Map<String, dynamic>> generateExcelTemplate() async {
    try {
      final excel = Excel.createExcel();

      // Get the first sheet
      final sheet = excel['Students Template'];

      // Add headers
      final headers = [
        'Student ID *',
        'First Name *',
        'Last Name *',
        'Email',
        'Phone',
        'Class ID (Optional)',
      ];

      for (int i = 0; i < headers.length; i++) {
        final cell =
            sheet.cell(CellIndex.indexByColumnRow(columnIndex: i, rowIndex: 0));
        cell.value = TextCellValue(headers[i]);

        // Make headers bold
        final cellStyle = CellStyle(
          bold: true,
          fontColorHex: ExcelColor.white,
          backgroundColorHex: ExcelColor.fromHexString('#4CAF50'),
        );
        cell.cellStyle = cellStyle;
      }

      // Add example data
      final exampleData = [
        ['STU001', 'John', 'Doe', 'john.doe@example.com', '+1234567890', '1'],
        [
          'STU002',
          'Jane',
          'Smith',
          'jane.smith@example.com',
          '+1234567891',
          '1'
        ],
        ['STU003', 'Mike', 'Johnson', 'mike.j@example.com', '+1234567892', ''],
      ];

      for (int row = 0; row < exampleData.length; row++) {
        for (int col = 0; col < exampleData[row].length; col++) {
          final cell = sheet.cell(
              CellIndex.indexByColumnRow(columnIndex: col, rowIndex: row + 1));
          cell.value = TextCellValue(exampleData[row][col]);

          // Style example data
          if (col < 3) {
            // Required fields
            final cellStyle = CellStyle(
              backgroundColorHex: ExcelColor.fromHexString('#E8F5E8'),
            );
            cell.cellStyle = cellStyle;
          }
        }
      }

      // Auto-adjust column widths
      for (int col = 0; col < headers.length; col++) {
        // sheet.setColWidth(col, 20); // setColWidth is deprecated/removed
      }

      return {
        'excel': excel,
        'fileName': 'students_template.xlsx',
      };
    } catch (e) {
      throw ExcelImportException('Failed to generate template: $e');
    }
  }
}

class ExcelImportResult {
  final int successCount;
  final int errorCount;
  final List<String> errors;
  final int totalProcessed;

  const ExcelImportResult({
    required this.successCount,
    required this.errorCount,
    required this.errors,
    required this.totalProcessed,
  });

  bool get hasErrors => errorCount > 0;
  bool get hasSuccess => successCount > 0;
  double get successRate =>
      totalProcessed > 0 ? successCount / totalProcessed : 0.0;

  @override
  String toString() {
    return 'ExcelImportResult(success: $successCount, errors: $errorCount, total: $totalProcessed)';
  }
}

class ExcelImportException implements Exception {
  final String message;

  const ExcelImportException(this.message);

  @override
  String toString() => 'ExcelImportException: $message';
}

// Provider for Excel import service
final excelImportServiceProvider = Provider<ExcelImportService>((ref) {
  return ExcelImportService();
});
