import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:googleapis_auth/auth_io.dart';
import 'package:googleapis/sheets/v4.dart' as sheets;
import 'package:logger/logger.dart';
import '../models/attendance.dart';
import '../models/student.dart';

class GoogleSheetsService {
  final Logger _logger = Logger();
  static const List<String> _scopes = [sheets.SheetsApi.spreadsheetsScope];

  Future<AuthClient?> _getClient() async {
    try {
      // In a real app, these would come from a secure source
      final clientId = 'your-client-id.apps.googleusercontent.com';
      final clientSecret = 'your-client-secret';

      final client = await clientViaUserConsent(
        ClientId(clientId, clientSecret),
        _scopes,
        (url) async {
          // In a real app, this would open the URL in browser or webview
          _logger.d('Please go to this URL to authenticate: $url');
        },
      );

      return client;
    } catch (e) {
      _logger.e('Error getting Google auth client: $e');
      return null;
    }
  }

  Future<String?> createSpreadsheet(
    String title,
    List<String> sheetNames,
  ) async {
    try {
      final client = await _getClient();
      if (client == null) return null;

      final sheetsApi = sheets.SheetsApi(client);

      final spreadsheet = sheets.Spreadsheet(
        properties: sheets.SpreadsheetProperties(
          title: title,
        ),
        sheets: sheetNames.map((name) {
          return sheets.Sheet(
            properties: sheets.SheetProperties(
              title: name,
            ),
          );
        }).toList(),
      );

      final result = await sheetsApi.spreadsheets.create(spreadsheet);
      client.close();

      return result.spreadsheetId;
    } catch (e) {
      _logger.e('Error creating spreadsheet: $e');
      return null;
    }
  }

  Future<bool> exportStudentsToSheet(
    String spreadsheetId,
    String sheetName,
    List<Student> students,
  ) async {
    try {
      final client = await _getClient();
      if (client == null) return false;

      final sheetsApi = sheets.SheetsApi(client);

      // Prepare data
      final headers = [
        'Student ID',
        'First Name',
        'Last Name',
        'Email',
        'Phone',
        'Class ID'
      ];
      final rows = [
        headers,
        ...students.map((student) {
          return [
            student.studentId,
            student.firstName,
            student.lastName,
            student.email ?? '',
            student.phone ?? '',
            student.classId?.toString() ?? '',
          ];
        }),
      ];

      // Write to sheet
      await sheetsApi.spreadsheets.values.update(
        sheets.ValueRange(
          values: rows,
        ),
        spreadsheetId,
        '$sheetName!A1:${_getColumnLetter(headers.length)}${rows.length}',
        valueInputOption: 'USER_ENTERED',
      );

      client.close();
      return true;
    } catch (e) {
      _logger.e('Error exporting students to sheet: $e');
      return false;
    }
  }

  Future<bool> exportAttendanceToSheet(
    String spreadsheetId,
    String sheetName,
    List<AttendanceRecord> records,
    List<Student> students,
  ) async {
    try {
      final client = await _getClient();
      if (client == null) return false;

      final sheetsApi = sheets.SheetsApi(client);

      // Prepare data
      final headers = [
        'Date',
        'Student ID',
        'Student Name',
        'Class ID',
        'Status',
        'Notes'
      ];

      final rows = records.map((record) {
        final student = students.firstWhere(
          (s) => s.id == record.studentId,
          orElse: () => Student(
            studentId: record.studentId.toString(),
            firstName: 'Unknown',
            lastName: 'Student',
          ),
        );

        return [
          record.date.toIso8601String().split('T')[0],
          record.studentId.toString(),
          student.displayName,
          record.classId.toString(),
          record.status.name,
          record.notes ?? '',
        ];
      }).toList();

      final allRows = [headers, ...rows];

      // Write to sheet
      await sheetsApi.spreadsheets.values.update(
        sheets.ValueRange(
          values: allRows,
        ),
        spreadsheetId,
        '$sheetName!A1:${_getColumnLetter(headers.length)}${allRows.length}',
        valueInputOption: 'USER_ENTERED',
      );

      client.close();
      return true;
    } catch (e) {
      _logger.e('Error exporting attendance to sheet: $e');
      return false;
    }
  }

  Future<List<Map<String, dynamic>>> importStudentsFromSheet(
    String spreadsheetId,
    String sheetName,
  ) async {
    try {
      final client = await _getClient();
      if (client == null) return [];

      final sheetsApi = sheets.SheetsApi(client);

      final result = await sheetsApi.spreadsheets.values.get(
        spreadsheetId,
        '$sheetName!A:F',
      );

      final rows = result.values ?? [];
      if (rows.isEmpty) return [];

      // Skip header row
      final dataRows = rows.skip(1);

      return dataRows.map((row) {
        return {
          'student_id': row[0]?.toString() ?? '',
          'first_name': row[1]?.toString() ?? '',
          'last_name': row[2]?.toString() ?? '',
          'email': row[3]?.toString() ?? '',
          'phone': row[4]?.toString() ?? '',
          'class_id': row[5]?.toString() ?? '',
        };
      }).where((student) {
        // Skip empty rows
        return student['student_id']!.isNotEmpty &&
            student['first_name']!.isNotEmpty &&
            student['last_name']!.isNotEmpty;
      }).toList();
    } catch (e) {
      _logger.e('Error importing students from sheet: $e');
      return [];
    }
  }

  Future<bool> updateAttendanceRecord(
    String spreadsheetId,
    String sheetName,
    AttendanceRecord record,
    Student student,
    int rowIndex,
  ) async {
    try {
      final client = await _getClient();
      if (client == null) return false;

      final sheetsApi = sheets.SheetsApi(client);

      final row = [
        record.date.toIso8601String().split('T')[0],
        record.studentId.toString(),
        student.displayName,
        record.classId.toString(),
        record.status.name,
        record.notes ?? '',
      ];

      await sheetsApi.spreadsheets.values.update(
        sheets.ValueRange(
          values: [row],
        ),
        spreadsheetId,
        '$sheetName!A$rowIndex:${_getColumnLetter(row.length)}$rowIndex',
        valueInputOption: 'USER_ENTERED',
      );

      client.close();
      return true;
    } catch (e) {
      _logger.e('Error updating attendance record: $e');
      return false;
    }
  }

  Future<String> createTemplateSpreadsheet() async {
    final title = 'AttendEase Template';
    final sheetNames = ['Students', 'Attendance', 'Classes'];

    final spreadsheetId = await createSpreadsheet(title, sheetNames);
    if (spreadsheetId == null) return '';

    final client = await _getClient();
    if (client == null) return '';

    try {
      final sheetsApi = sheets.SheetsApi(client);

      // Create Students template
      await sheetsApi.spreadsheets.values.update(
        sheets.ValueRange(
          values: [
            [
              'Student ID',
              'First Name',
              'Last Name',
              'Email',
              'Phone',
              'Class ID'
            ]
          ],
        ),
        spreadsheetId,
        'Students!A1:F1',
        valueInputOption: 'USER_ENTERED',
      );

      // Create Attendance template
      await sheetsApi.spreadsheets.values.update(
        sheets.ValueRange(
          values: [
            [
              'Date',
              'Student ID',
              'Student Name',
              'Class ID',
              'Status',
              'Notes'
            ]
          ],
        ),
        spreadsheetId,
        'Attendance!A1:F1',
        valueInputOption: 'USER_ENTERED',
      );

      // Create Classes template
      await sheetsApi.spreadsheets.values.update(
        sheets.ValueRange(
          values: [
            ['Class Name', 'Section', 'School Year', 'Description', 'Active']
          ],
        ),
        spreadsheetId,
        'Classes!A1:E1',
        valueInputOption: 'USER_ENTERED',
      );

      client.close();
      return 'https://docs.google.com/spreadsheets/d/$spreadsheetId';
    } catch (e) {
      client.close();
      _logger.e('Error creating template spreadsheet: $e');
      return '';
    }
  }

  String _getColumnLetter(int columnNumber) {
    String column = '';
    while (columnNumber > 0) {
      columnNumber--;
      column = String.fromCharCode(65 + (columnNumber % 26)) + column;
      columnNumber ~/= 26;
    }
    return column;
  }
}

class GoogleSheetsIntegrationResult {
  final bool success;
  final String? spreadsheetId;
  final String? spreadsheetUrl;
  final String? error;

  const GoogleSheetsIntegrationResult({
    required this.success,
    this.spreadsheetId,
    this.spreadsheetUrl,
    this.error,
  });

  factory GoogleSheetsIntegrationResult.success(
    String spreadsheetId,
    String spreadsheetUrl,
  ) {
    return GoogleSheetsIntegrationResult(
      success: true,
      spreadsheetId: spreadsheetId,
      spreadsheetUrl: spreadsheetUrl,
    );
  }

  factory GoogleSheetsIntegrationResult.failure(String error) {
    return GoogleSheetsIntegrationResult(
      success: false,
      error: error,
    );
  }
}

// Provider for Google Sheets service
final googleSheetsServiceProvider = Provider<GoogleSheetsService>((ref) {
  return GoogleSheetsService();
});
