import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:excel/excel.dart' hide Border;
import 'package:path_provider/path_provider.dart';
import '../../models/attendance.dart';

import '../../providers/attendance_provider.dart';

import '../../widgets/common/loading_widget.dart';
import '../../widgets/common/error_widget.dart';

class AttendanceListScreen extends ConsumerStatefulWidget {
  const AttendanceListScreen({super.key});

  @override
  ConsumerState<AttendanceListScreen> createState() =>
      _AttendanceListScreenState();
}

class _AttendanceListScreenState extends ConsumerState<AttendanceListScreen> {
  DateTime? startDate;
  DateTime? endDate;
  int? selectedClassId;
  String selectedStatusFilter = 'all';

  @override
  void initState() {
    super.initState();
    _loadAttendance();
  }

  Future<void> _loadAttendance() async {
    await ref.read(attendanceProvider.notifier).loadAttendance(
          classId: selectedClassId,
          date: startDate, // Changed to date parameter
        );
  }

  void _onFilterChanged() {
    _loadAttendance();
  }

  void _onDateRangeSelected() async {
    final DateTimeRange? picked = await showDateRangePicker(
      context: context,
      firstDate: DateTime(2020),
      lastDate: DateTime.now().add(const Duration(days: 30)),
      initialDateRange: startDate != null && endDate != null
          ? DateTimeRange(start: startDate!, end: endDate!)
          : null,
    );

    if (picked != null) {
      setState(() {
        startDate = picked.start;
        endDate = picked.end;
      });
      _onFilterChanged();
    }
  }

  @override
  Widget build(BuildContext context) {
    final attendanceState = ref.watch(attendanceProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Attendance Records'),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
        actions: [
          IconButton(
            onPressed: _exportToExcel,
            icon: const Icon(Icons.file_download),
            tooltip: 'Export to Excel',
          ),
        ],
      ),
      body: Column(
        children: [
          _buildFilters(),
          if (attendanceState.isLoading)
            const Expanded(child: LoadingWidget())
          else if (attendanceState.error != null)
            Expanded(
              child: CommonErrorWidget(
                error: attendanceState.error!,
                onRetry: _loadAttendance,
              ),
            )
          else
            Expanded(
              child: attendanceState.records.isEmpty
                  ? _buildEmptyState()
                  : _buildAttendanceList(),
            ),
        ],
      ),
    );
  }

  Widget _buildFilters() {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.1),
            blurRadius: 4,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: InkWell(
                  onTap: _onDateRangeSelected,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 12, vertical: 16),
                    decoration: BoxDecoration(
                      border: Border.all(color: Colors.grey),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.date_range),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            startDate != null && endDate != null
                                ? '${DateFormat('MMM dd').format(startDate!)} - ${DateFormat('MMM dd, yyyy').format(endDate!)}'
                                : 'Select Date Range',
                            style: Theme.of(context).textTheme.bodyLarge,
                          ),
                        ),
                        const Icon(Icons.arrow_drop_down),
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: DropdownButtonFormField<String>(
                  initialValue: selectedStatusFilter,
                  decoration: const InputDecoration(
                    labelText: 'Status',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.filter_list),
                  ),
                  items: const [
                    DropdownMenuItem(value: 'all', child: Text('All')),
                    DropdownMenuItem(value: 'present', child: Text('Present')),
                    DropdownMenuItem(value: 'absent', child: Text('Absent')),
                    DropdownMenuItem(value: 'late', child: Text('Late')),
                    DropdownMenuItem(value: 'excused', child: Text('Excused')),
                  ],
                  onChanged: (value) {
                    if (value != null) {
                      setState(() {
                        selectedStatusFilter = value;
                      });
                      _onFilterChanged();
                    }
                  },
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            '${ref.watch(attendanceProvider).records.length} records found',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.primary,
                ),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.assignment_outlined,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            'No attendance records found',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          Text(
            'Try adjusting your filters or record some attendance',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
        ],
      ),
    );
  }

  Widget _buildAttendanceList() {
    final attendanceState = ref.watch(attendanceProvider);

    // Filter records based on status
    List<AttendanceRecord> filteredRecords = attendanceState.records;
    if (selectedStatusFilter != 'all') {
      final status = AttendanceStatus.values.firstWhere(
        (s) => s.name == selectedStatusFilter,
      );
      filteredRecords = attendanceState.records
          .where((record) => record.status == status)
          .toList();
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: filteredRecords.length,
      itemBuilder: (context, index) {
        final record = filteredRecords[index];
        return _buildAttendanceCard(record);
      },
    );
  }

  Widget _buildAttendanceCard(AttendanceRecord record) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      elevation: 2,
      child: ListTile(
        contentPadding: const EdgeInsets.all(16),
        leading: CircleAvatar(
          backgroundColor:
              _getStatusColor(record.status).withValues(alpha: 0.2),
          child: Icon(
            _getStatusIcon(record.status),
            color: _getStatusColor(record.status),
          ),
        ),
        title: Text(
          'Student ID: ${record.studentId}',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const SizedBox(height: 4),
            Text(
              'Class ID: ${record.classId}',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            Text(
              'Date: ${DateFormat('MMM dd, yyyy').format(record.date)}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            if (record.notes != null && record.notes!.isNotEmpty) ...[
              const SizedBox(height: 4),
              Text(
                'Notes: ${record.notes}',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      fontStyle: FontStyle.italic,
                    ),
              ),
            ],
          ],
        ),
        trailing: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          decoration: BoxDecoration(
            color: _getStatusColor(record.status).withValues(alpha: 0.1),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(
              color: _getStatusColor(record.status).withValues(alpha: 0.3),
            ),
          ),
          child: Text(
            record.status.name.toUpperCase(),
            style: TextStyle(
              color: _getStatusColor(record.status),
              fontWeight: FontWeight.bold,
              fontSize: 12,
            ),
          ),
        ),
        onTap: () => _showAttendanceDetails(record),
      ),
    );
  }

  IconData _getStatusIcon(AttendanceStatus status) {
    switch (status) {
      case AttendanceStatus.present:
        return Icons.check_circle;
      case AttendanceStatus.absent:
        return Icons.cancel;
      case AttendanceStatus.late:
        return Icons.access_time;
      case AttendanceStatus.excused:
        return Icons.info;
    }
  }

  Color _getStatusColor(AttendanceStatus status) {
    switch (status) {
      case AttendanceStatus.present:
        return Colors.green;
      case AttendanceStatus.absent:
        return Colors.red;
      case AttendanceStatus.late:
        return Colors.orange;
      case AttendanceStatus.excused:
        return Colors.blue;
    }
  }

  void _showAttendanceDetails(AttendanceRecord record) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (context) => AttendanceDetailsSheet(record: record),
    );
  }

  Future<void> _exportToExcel() async {
    try {
      // Simple export implementation for now
      final attendanceState = ref.read(attendanceProvider);

      if (attendanceState.records.isEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('No attendance data to export'),
            backgroundColor: Colors.orange,
          ),
        );
        return;
      }

      // Create Excel file
      final excelFile = Excel.createExcel();
      final Sheet sheet = excelFile['Attendance Report'];

      // Add headers
      sheet.cell(CellIndex.indexByString('A1')).value = TextCellValue('Date');
      sheet.cell(CellIndex.indexByString('B1')).value =
          TextCellValue('Student ID');
      sheet.cell(CellIndex.indexByString('C1')).value =
          TextCellValue('Student Name');
      sheet.cell(CellIndex.indexByString('D1')).value = TextCellValue('Class');
      sheet.cell(CellIndex.indexByString('E1')).value = TextCellValue('Status');
      sheet.cell(CellIndex.indexByString('F1')).value = TextCellValue('Notes');

      // Add data rows
      for (int i = 0; i < attendanceState.records.length; i++) {
        final record = attendanceState.records[i];
        final rowIndex = i + 2;

        sheet.cell(CellIndex.indexByString('A$rowIndex')).value =
            TextCellValue(record.date.toIso8601String().split('T')[0]);
        sheet.cell(CellIndex.indexByString('B$rowIndex')).value =
            TextCellValue(record.studentId.toString());
        sheet.cell(CellIndex.indexByString('C$rowIndex')).value =
            TextCellValue(record.student?.displayName ?? 'N/A');
        sheet.cell(CellIndex.indexByString('D$rowIndex')).value =
            TextCellValue(record.classRecord?.name ?? 'N/A');
        sheet.cell(CellIndex.indexByString('E$rowIndex')).value =
            TextCellValue(record.status.name.toUpperCase());
        sheet.cell(CellIndex.indexByString('F$rowIndex')).value =
            TextCellValue(record.notes ?? '');
      }

      // Save the file
      final directory = await getApplicationDocumentsDirectory();
      final fileName =
          'attendance_report_${DateTime.now().toIso8601String().split('T')[0]}.xlsx';
      final file = File('${directory.path}/$fileName');

      await file.writeAsBytes(excelFile.save()!);

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Export completed: Saved to $fileName'),
            backgroundColor: Colors.green,
          ),
        );
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Export failed: ${error.toString()}'),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }
}

class AttendanceDetailsSheet extends ConsumerWidget {
  final AttendanceRecord record;

  const AttendanceDetailsSheet({
    super.key,
    required this.record,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'Attendance Details',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              IconButton(
                onPressed: () => Navigator.of(context).pop(),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
          const SizedBox(height: 16),
          _buildDetailRow('Student ID', record.studentId.toString()),
          _buildDetailRow('Class ID', record.classId.toString()),
          _buildDetailRow(
              'Date', DateFormat('EEEE, MMMM dd, yyyy').format(record.date)),
          _buildDetailRow('Status', record.status.name.toUpperCase()),
          if (record.notes != null && record.notes!.isNotEmpty)
            _buildDetailRow('Notes', record.notes!),
          if (record.createdAt != null)
            _buildDetailRow('Created',
                DateFormat('MMM dd, yyyy HH:mm').format(record.createdAt!)),
          if (record.updatedAt != null)
            _buildDetailRow('Updated',
                DateFormat('MMM dd, yyyy HH:mm').format(record.updatedAt!)),
          const SizedBox(height: 24),
          Row(
            children: [
              Expanded(
                child: ElevatedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                    _showEditDialog(context, ref);
                  },
                  icon: const Icon(Icons.edit),
                  label: const Text('Edit'),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                    _deleteAttendance(context, ref);
                  },
                  icon: const Icon(Icons.delete, color: Colors.red),
                  label:
                      const Text('Delete', style: TextStyle(color: Colors.red)),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildDetailRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 100,
            child: Text(
              '$label:',
              style: const TextStyle(
                fontWeight: FontWeight.bold,
                color: Colors.grey,
              ),
            ),
          ),
          Expanded(
            child: Text(value),
          ),
        ],
      ),
    );
  }

  void _showEditDialog(BuildContext context, WidgetRef ref) {
    final record = this.record;
    final statusController = TextEditingController(text: record.status.name);
    final notesController = TextEditingController(text: record.notes ?? '');

    showDialog(
      context: context,
      builder: (dialogContext) => Consumer(
        builder: (context, ref, child) {
          return AlertDialog(
            title: const Text('Edit Attendance'),
            content: SizedBox(
              width: 400,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text('Student: ${record.student?.displayName ?? 'Unknown'}'),
                  const SizedBox(height: 8),
                  Text(
                      'Date: ${DateFormat('MMM dd, yyyy').format(record.date)}'),
                  const SizedBox(height: 16),
                  DropdownButtonFormField<AttendanceStatus>(
                    initialValue: record.status,
                    decoration: const InputDecoration(
                      labelText: 'Status',
                      border: OutlineInputBorder(),
                    ),
                    items: AttendanceStatus.values.map((status) {
                      return DropdownMenuItem(
                        value: status,
                        child: Text(status.name.toUpperCase()),
                      );
                    }).toList(),
                    onChanged: (value) {
                      if (value != null) {
                        statusController.text = value.name;
                      }
                    },
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: notesController,
                    decoration: const InputDecoration(
                      labelText: 'Notes',
                      border: OutlineInputBorder(),
                    ),
                    maxLines: 3,
                  ),
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: const Text('Cancel'),
              ),
              TextButton(
                onPressed: () {
                  final updatedRecord = AttendanceRecord(
                    id: record.id,
                    studentId: record.studentId,
                    classId: record.classId,
                    date: record.date,
                    status: AttendanceStatus.values.firstWhere(
                      (status) => status.name == statusController.text,
                      orElse: () => AttendanceStatus.absent,
                    ),
                    notes: notesController.text.trim().isEmpty
                        ? null
                        : notesController.text.trim(),
                    createdAt: record.createdAt,
                    updatedAt: DateTime.now(),
                  );

                  final attendanceNotifier =
                      ref.read(attendanceProvider.notifier);
                  attendanceNotifier.updateAttendanceRecord(
                      updatedRecord.id!, updatedRecord.status,
                      notes: updatedRecord.notes);

                  Navigator.of(dialogContext).pop();
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text('Attendance record updated'),
                      backgroundColor: Colors.green,
                    ),
                  );
                },
                child: const Text('Save'),
              ),
            ],
          );
        },
      ),
    );
  }

  void _deleteAttendance(BuildContext context, WidgetRef ref) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Record'),
        content: const Text(
            'Are you sure you want to delete this attendance record?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              Navigator.of(context).pop();
              if (record.id != null) {
                ref
                    .read(attendanceProvider.notifier)
                    .deleteAttendanceRecord(record.id!);
              }
            },
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
  }
}
