import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import '../../services/excel_import_service.dart';
import '../../providers/student_provider.dart';
import '../../providers/class_provider.dart';

class ExcelImportScreen extends ConsumerStatefulWidget {
  const ExcelImportScreen({super.key});

  @override
  ConsumerState<ExcelImportScreen> createState() => _ExcelImportScreenState();
}

class _ExcelImportScreenState extends ConsumerState<ExcelImportScreen> {
  File? selectedFile;
  int? selectedClassId;
  bool isImporting = false;
  ExcelImportResult? importResult;

  @override
  void initState() {
    super.initState();
    _loadClasses();
  }

  Future<void> _loadClasses() async {
    await ref.read(classProvider.notifier).loadClasses();
  }

  Future<void> _pickFile() async {
    final excelImportService = ExcelImportService();
    final file = await excelImportService.pickExcelFile();

    if (file != null) {
      setState(() {
        selectedFile = file;
        importResult = null;
      });
    }
  }

  Future<void> _downloadTemplate() async {
    try {
      final excelImportService = ExcelImportService();
      final templateData = await excelImportService.generateExcelTemplate();

      final directory = await getApplicationDocumentsDirectory();
      final file = File('${directory.path}/${templateData['fileName']}');

      final excel = templateData['excel'];
      final fileBytes = excel.save();
      await file.writeAsBytes(fileBytes!);

      if (mounted) {
        _showSnackBar('Template downloaded to: ${file.path}');
      }
    } catch (e) {
      _showSnackBar('Failed to download template: $e', isError: true);
    }
  }

  Future<void> _importStudents() async {
    if (selectedFile == null) {
      _showSnackBar('Please select an Excel file first', isError: true);
      return;
    }

    setState(() {
      isImporting = true;
    });

    try {
      final excelImportService = ExcelImportService();

      // Parse students from Excel
      final students = await excelImportService.parseStudentsFromExcel(
        selectedFile!,
        classId: selectedClassId,
      );

      if (students.isEmpty) {
        _showSnackBar('No valid students found in the Excel file',
            isError: true);
        setState(() {
          isImporting = false;
        });
        return;
      }

      // Import students
      final result = await excelImportService.importStudents(
        students,
        ref.read(studentProvider.notifier),
        ref.read(classProvider.notifier),
      );

      setState(() {
        importResult = result;
        isImporting = false;
      });

      _showImportResultDialog();
    } catch (e) {
      setState(() {
        isImporting = false;
      });
      _showSnackBar('Import failed: $e', isError: true);
    }
  }

  void _showImportResultDialog() {
    if (importResult == null) return;

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Import Complete'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              importResult!.hasErrors ? Icons.warning : Icons.check_circle,
              size: 64,
              color: importResult!.hasErrors ? Colors.orange : Colors.green,
            ),
            const SizedBox(height: 16),
            Text(
              'Import finished successfully!',
              style: Theme.of(context).textTheme.titleLarge,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              '${importResult!.successCount} of ${importResult!.totalProcessed} students imported',
              style: Theme.of(context).textTheme.bodyMedium,
              textAlign: TextAlign.center,
            ),
            if (importResult!.hasErrors) ...[
              const SizedBox(height: 16),
              Text(
                '${importResult!.errorCount} errors occurred:',
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Colors.red,
                      fontWeight: FontWeight.bold,
                    ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Container(
                constraints: const BoxConstraints(maxHeight: 200),
                child: SingleChildScrollView(
                  child: Text(
                    importResult!.errors.join('\n'),
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ),
              ),
            ],
          ],
        ),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.of(context).pop();
              if (importResult!.hasSuccess) {
                Navigator.of(context).pop(); // Close import screen
              }
            },
            child: const Text('OK'),
          ),
        ],
      ),
    );
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
        title: const Text('Import Students from Excel'),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _buildInfoCard(),
            const SizedBox(height: 16),
            _buildClassSelection(),
            const SizedBox(height: 16),
            _buildFileSelection(),
            const SizedBox(height: 16),
            _buildImportButton(),
            const Spacer(),
            _buildTemplateSection(),
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
                  'Import Instructions',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            _buildInfoItem('1. Download the Excel template'),
            _buildInfoItem('2. Fill in student information'),
            _buildInfoItem(
                '3. Required fields: Student ID, First Name, Last Name'),
            _buildInfoItem('4. Optional fields: Email, Phone, Class ID'),
            _buildInfoItem('5. Upload the completed file'),
          ],
        ),
      ),
    );
  }

  Widget _buildInfoItem(String text) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Text(
        text,
        style: Theme.of(context).textTheme.bodySmall,
      ),
    );
  }

  Widget _buildClassSelection() {
    final classState = ref.watch(classProvider);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Assign to Class (Optional)',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
            ),
            const SizedBox(height: 8),
            DropdownButtonFormField<int>(
              initialValue: selectedClassId,
              decoration: const InputDecoration(
                labelText: 'Select Class',
                border: OutlineInputBorder(),
                prefixIcon: Icon(Icons.class_),
                helperText: 'Leave empty to import without class assignment',
              ),
              items: [
                const DropdownMenuItem<int>(
                  value: null,
                  child: Text('No Class Assignment'),
                ),
                ...classState.classes.map((class_) {
                  return DropdownMenuItem<int>(
                    value: class_.id,
                    child: Text(class_.displayName),
                  );
                }),
              ],
              onChanged: (value) {
                setState(() {
                  selectedClassId = value;
                });
              },
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildFileSelection() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Select Excel File',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
            ),
            const SizedBox(height: 8),
            if (selectedFile != null) ...[
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  border: Border.all(color: Colors.green),
                  borderRadius: BorderRadius.circular(8),
                  color: Colors.green.withValues(alpha: 0.1),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.file_present, color: Colors.green),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            selectedFile!.path.split('/').last,
                            style: const TextStyle(
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                          Text(
                            'File selected and ready for import',
                            style:
                                Theme.of(context).textTheme.bodySmall?.copyWith(
                                      color: Colors.green,
                                    ),
                          ),
                        ],
                      ),
                    ),
                    IconButton(
                      onPressed: () {
                        setState(() {
                          selectedFile = null;
                          importResult = null;
                        });
                      },
                      icon: const Icon(Icons.close, color: Colors.red),
                    ),
                  ],
                ),
              ),
            ] else ...[
              InkWell(
                onTap: _pickFile,
                child: Container(
                  padding: const EdgeInsets.all(32),
                  decoration: BoxDecoration(
                    border: Border.all(color: Colors.grey),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    children: [
                      Icon(
                        Icons.cloud_upload,
                        size: 48,
                        color: Theme.of(context).colorScheme.primary,
                      ),
                      const SizedBox(height: 8),
                      Text(
                        'Click to select Excel file',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      Text(
                        'Supports .xlsx and .xls files',
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context)
                                  .colorScheme
                                  .onSurfaceVariant,
                            ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildImportButton() {
    return ElevatedButton.icon(
      onPressed: isImporting || selectedFile == null ? null : _importStudents,
      icon: isImporting
          ? const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : const Icon(Icons.file_upload),
      label: Text(isImporting ? 'Importing...' : 'Import Students'),
      style: ElevatedButton.styleFrom(
        padding: const EdgeInsets.all(16),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
      ),
    );
  }

  Widget _buildTemplateSection() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.download,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Download Template',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              "Download our Excel template to ensure your data is formatted correctly. The template includes example data and formatting guidelines.",
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: _downloadTemplate,
              icon: const Icon(Icons.file_download),
              label: const Text('Download Template'),
            ),
          ],
        ),
      ),
    );
  }
}
