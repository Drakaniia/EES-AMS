import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart';
import '../../services/google_sheets_service.dart';
import '../../providers/student_provider.dart';
import '../../providers/attendance_provider.dart';
import '../../widgets/common/loading_widget.dart';

class GoogleSheetsIntegrationScreen extends ConsumerStatefulWidget {
  const GoogleSheetsIntegrationScreen({super.key});

  @override
  ConsumerState<GoogleSheetsIntegrationScreen> createState() =>
      _GoogleSheetsIntegrationScreenState();
}

class _GoogleSheetsIntegrationScreenState
    extends ConsumerState<GoogleSheetsIntegrationScreen> {
  String? _spreadsheetId;
  String? _spreadsheetUrl;
  bool _isLoading = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadSavedSpreadsheet();
  }

  Future<void> _loadSavedSpreadsheet() async {
    final prefs = await SharedPreferences.getInstance();
    final savedSpreadsheetId = prefs.getString('google_sheets_spreadsheet_id');
    
    if (savedSpreadsheetId != null && savedSpreadsheetId.isNotEmpty) {
      setState(() {
        _spreadsheetId = savedSpreadsheetId;
        _spreadsheetUrl = 'https://docs.google.com/spreadsheets/d/$savedSpreadsheetId';
      });
    }
  }

  Future<void> _createNewSpreadsheet() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      final googleSheetsService = GoogleSheetsService();
      final url = await googleSheetsService.createTemplateSpreadsheet();

      if (url.isNotEmpty) {
        final spreadsheetId = url.split('/').last; // Extract ID from URL
        final prefs = await SharedPreferences.getInstance();
        await prefs.setString('google_sheets_spreadsheet_id', spreadsheetId);
        
        setState(() {
          _spreadsheetUrl = url;
          _spreadsheetId = spreadsheetId;
        });
        _showSnackBar('Template spreadsheet created successfully');

        // Open the spreadsheet in browser
        await _launchUrl(url);
      } else {
        _showSnackBar('Failed to create spreadsheet', isError: true);
      }
    } catch (e) {
      setState(() {
        _error = 'Failed to create spreadsheet: $e';
      });
      _showSnackBar('Error: $e', isError: true);
    } finally {
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _exportStudents() async {
    if (_spreadsheetId == null) {
      _showSnackBar('Please connect to a Google Sheet first', isError: true);
      return;
    }

    setState(() {
      _isLoading = true;
    });

    try {
      final studentState = ref.read(studentProvider);
      if (studentState.students.isEmpty) {
        _showSnackBar('No students to export', isError: true);
        return;
      }

      final googleSheetsService = GoogleSheetsService();
      final success = await googleSheetsService.exportStudentsToSheet(
        _spreadsheetId!,
        'Students',
        studentState.students,
      );

      if (success) {
        _showSnackBar('Students exported successfully');
      } else {
        _showSnackBar('Failed to export students', isError: true);
      }
    } catch (e) {
      _showSnackBar('Error exporting students: $e', isError: true);
    } finally {
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _exportAttendance() async {
    if (_spreadsheetId == null) {
      _showSnackBar('Please connect to a Google Sheet first', isError: true);
      return;
    }

    setState(() {
      _isLoading = true;
    });

    try {
      final attendanceState = ref.read(attendanceProvider);
      final studentState = ref.read(studentProvider);

      if (attendanceState.records.isEmpty) {
        _showSnackBar('No attendance records to export', isError: true);
        return;
      }

      final googleSheetsService = GoogleSheetsService();
      final success = await googleSheetsService.exportAttendanceToSheet(
        _spreadsheetId!,
        'Attendance',
        attendanceState.records,
        studentState.students,
      );

      if (success) {
        _showSnackBar('Attendance records exported successfully');
      } else {
        _showSnackBar('Failed to export attendance records', isError: true);
      }
    } catch (e) {
      _showSnackBar('Error exporting attendance: $e', isError: true);
    } finally {
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _importStudents() async {
    if (_spreadsheetId == null) {
      _showSnackBar('Please connect to a Google Sheet first', isError: true);
      return;
    }

    setState(() {
      _isLoading = true;
    });

    try {
      final googleSheetsService = GoogleSheetsService();
      final studentsData = await googleSheetsService.importStudentsFromSheet(
        _spreadsheetId!,
        'Students',
      );

      if (studentsData.isEmpty) {
        _showSnackBar('No students found in the sheet', isError: true);
        return;
      }

      // Import students using the student provider
      for (final studentData in studentsData) {
        await ref.read(studentProvider.notifier).createStudent(studentData);
      }

      _showSnackBar('Successfully imported ${studentsData.length} students');
    } catch (e) {
      _showSnackBar('Error importing students: $e', isError: true);
    } finally {
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _launchUrl(String url) async {
    final uri = Uri.parse(url);
    if (await canLaunchUrl(uri)) {
      await launchUrl(uri);
    } else {
      _showSnackBar('Could not open URL', isError: true);
    }
  }

  void _showSnackBar(String message, {bool isError = false}) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        backgroundColor: isError ? Colors.red : Colors.green,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Google Sheets Integration'),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
      ),
      body: _isLoading ? const LoadingWidget() : _buildContent(),
    );
  }

  Widget _buildContent() {
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.error_outline,
              size: 64,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(height: 16),
            Text(
              'Error',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 8),
            Text(
              _error!,
              style: Theme.of(context).textTheme.bodyMedium,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 24),
            ElevatedButton(
              onPressed: () => setState(() => _error = null),
              child: const Text('Try Again'),
            ),
          ],
        ),
      );
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildConnectionCard(),
          const SizedBox(height: 16),
          _buildExportActionsCard(),
          const SizedBox(height: 16),
          _buildImportActionsCard(),
          const SizedBox(height: 16),
          _buildInfoCard(),
        ],
      ),
    );
  }

  Widget _buildConnectionCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.link,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Spreadsheet Connection',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            if (_spreadsheetUrl != null) ...[
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  border: Border.all(color: Colors.green),
                  borderRadius: BorderRadius.circular(8),
                  color: Colors.green.withValues(alpha: 0.1),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(
                          Icons.check_circle,
                          color: Colors.green,
                          size: 20,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'Connected to spreadsheet',
                            style: const TextStyle(
                              fontWeight: FontWeight.bold,
                              color: Colors.green,
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'Spreadsheet ID: ${_spreadsheetId ?? 'Unknown'}',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        Expanded(
                          child: TextButton.icon(
                            onPressed: () => _launchUrl(_spreadsheetUrl!),
                            icon: const Icon(Icons.open_in_browser),
                            label: const Text('Open Sheet'),
                          ),
                        ),
                        TextButton.icon(
                          onPressed: () async {
                            final prefs = await SharedPreferences.getInstance();
                            await prefs.remove('google_sheets_spreadsheet_id');
                            
                            setState(() {
                              _spreadsheetId = null;
                              _spreadsheetUrl = null;
                            });
                          },
                          icon: const Icon(Icons.link_off),
                          label: const Text('Disconnect'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ] else ...[
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  border: Border.all(color: Colors.grey),
                  borderRadius: BorderRadius.circular(8),
                  color: Colors.grey.withValues(alpha: 0.1),
                ),
                child: Column(
                  children: [
                    Icon(
                      Icons.cloud_off,
                      color: Colors.grey,
                      size: 48,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      'Not connected to any spreadsheet',
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                            color: Colors.grey,
                          ),
                    ),
                    const SizedBox(height: 12),
                    ElevatedButton.icon(
                      onPressed: _createNewSpreadsheet,
                      icon: const Icon(Icons.add),
                      label: const Text('Create New Spreadsheet'),
                    ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildExportActionsCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.file_download,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Export Actions',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: ElevatedButton.icon(
                onPressed: _spreadsheetId != null ? _exportStudents : null,
                icon: const Icon(Icons.people),
                label: const Text('Export Students'),
              ),
            ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: ElevatedButton.icon(
                onPressed: _spreadsheetId != null ? _exportAttendance : null,
                icon: const Icon(Icons.assignment),
                label: const Text('Export Attendance'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildImportActionsCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.file_upload,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Import Actions',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: ElevatedButton.icon(
                onPressed: _spreadsheetId != null ? _importStudents : null,
                icon: const Icon(Icons.people),
                label: const Text('Import Students'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: Colors.orange,
                  foregroundColor: Colors.white,
                ),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Note: Importing will create new records. Duplicates may be created if students already exist.',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildInfoCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.info_outline,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'About Google Sheets Integration',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            _buildInfoItem('Real-time Sync',
                'Sync data between AttendEase and Google Sheets in real-time'),
            _buildInfoItem('Backup & Restore',
                'Use Google Sheets as a backup for your attendance data'),
            _buildInfoItem(
                'Collaboration', 'Share spreadsheets with other staff members'),
            _buildInfoItem('Data Analysis',
                'Use Google Sheets\' built-in tools for data analysis'),
            const SizedBox(height: 16),
            const Text(
              'The spreadsheet will contain three sheets:',
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            _buildInfoItem(
                'Students', 'Student information and class assignments'),
            _buildInfoItem('Attendance', 'Daily attendance records'),
            _buildInfoItem('Classes', 'Class information and settings'),
          ],
        ),
      ),
    );
  }

  Widget _buildInfoItem(String title, String description) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            Icons.check_circle,
            size: 16,
            color: Theme.of(context).colorScheme.primary,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: const TextStyle(fontWeight: FontWeight.bold),
                ),
                Text(
                  description,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
