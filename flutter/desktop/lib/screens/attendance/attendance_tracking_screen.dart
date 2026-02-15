import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';
import '../../models/attendance.dart';
import '../../providers/attendance_provider.dart';
import '../../providers/student_provider.dart';
import '../../providers/class_provider.dart';
import '../../widgets/common/loading_widget.dart';
import '../../widgets/common/error_widget.dart';

class AttendanceTrackingScreen extends ConsumerStatefulWidget {
  const AttendanceTrackingScreen({super.key});

  @override
  ConsumerState<AttendanceTrackingScreen> createState() =>
      _AttendanceTrackingScreenState();
}

class _AttendanceTrackingScreenState
    extends ConsumerState<AttendanceTrackingScreen> {
  int? selectedClassId;
  DateTime selectedDate = DateTime.now();
  Map<int, AttendanceStatus> attendanceData = {};

  @override
  void initState() {
    super.initState();
    _loadInitialData();
  }

  Future<void> _loadInitialData() async {
    await ref.read(classProvider.notifier).loadClasses();
    if (selectedClassId != null) {
      await _loadStudentsForClass();
      await _loadExistingAttendance();
    }
  }

  Future<void> _loadStudentsForClass() async {
    if (selectedClassId == null) return;
    await ref
        .read(studentProvider.notifier)
        .loadStudents(classId: selectedClassId);
  }

  Future<void> _loadExistingAttendance() async {
    if (selectedClassId == null) return;
    await ref.read(attendanceProvider.notifier).loadAttendance(
          classId: selectedClassId,
          date: selectedDate,
        );
  }

  void _onClassChanged(int? classId) {
    setState(() {
      selectedClassId = classId;
      attendanceData.clear();
    });
    _loadStudentsForClass();
    _loadExistingAttendance();
  }

  void _onDateChanged(DateTime date) {
    setState(() {
      selectedDate = date;
      attendanceData.clear();
    });
    _loadExistingAttendance();
  }

  void _onAttendanceStatusChanged(int studentId, AttendanceStatus status) {
    setState(() {
      attendanceData[studentId] = status;
    });
  }

  Future<void> _saveAttendance() async {
    if (selectedClassId == null || attendanceData.isEmpty) {
      _showSnackBar(
          'Please select a class and mark attendance for at least one student',
          isError: true);
      return;
    }

    await ref.read(attendanceProvider.notifier).markAttendance(
          selectedClassId!,
          selectedDate,
          attendanceData,
        );

    final state = ref.read(attendanceProvider);
    if (state.error != null) {
      _showSnackBar(state.error!, isError: true);
    } else {
      _showSnackBar('Attendance saved successfully');
      setState(() {
        attendanceData.clear();
      });
      await _loadExistingAttendance();
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
    final classState = ref.watch(classProvider);
    final studentState = ref.watch(studentProvider);
    final attendanceState = ref.watch(attendanceProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Attendance Tracking'),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
        actions: [
          IconButton(
            onPressed: _saveAttendance,
            icon: const Icon(Icons.save),
          ),
        ],
      ),
      body: Column(
        children: [
          _buildHeader(),
          if (classState.isLoading ||
              studentState.isLoading ||
              attendanceState.isLoading)
            const Expanded(child: LoadingWidget())
          else if (classState.error != null ||
              studentState.error != null ||
              attendanceState.error != null)
            Expanded(
              child: CommonErrorWidget(
                error: classState.error ??
                    studentState.error ??
                    attendanceState.error!,
                onRetry: _loadInitialData,
              ),
            )
          else
            Expanded(child: _buildAttendanceList()),
        ],
      ),
    );
  }

  Widget _buildHeader() {
    final classState = ref.watch(classProvider);
    final studentState = ref.watch(studentProvider);

    return Container(
      padding: const EdgeInsets.all(16),
      color: Theme.of(context).colorScheme.surface,
      child: Column(
        children: [
          Row(
            children: [
              Expanded(
                child: DropdownButtonFormField<int>(
                  initialValue: selectedClassId,
                  decoration: const InputDecoration(
                    labelText: 'Select Class',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.class_),
                  ),
                  items: classState.classes.map((class_) {
                    return DropdownMenuItem<int>(
                      value: class_.id,
                      child: Text(class_.displayName),
                    );
                  }).toList(),
                  onChanged: _onClassChanged,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: InkWell(
                  onTap: () async {
                    final date = await showDatePicker(
                      context: context,
                      initialDate: selectedDate,
                      firstDate: DateTime(2020),
                      lastDate: DateTime.now().add(const Duration(days: 30)),
                    );
                    if (date != null) {
                      _onDateChanged(date);
                    }
                  },
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 12, vertical: 16),
                    decoration: BoxDecoration(
                      border: Border.all(color: Colors.grey),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.calendar_today),
                        const SizedBox(width: 8),
                        Text(DateFormat('MMM dd, yyyy').format(selectedDate)),
                        const Spacer(),
                        const Icon(Icons.arrow_drop_down),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          if (selectedClassId != null)
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'Mark attendance for ${studentState.students.length} students',
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: Theme.of(context).colorScheme.primary,
                      ),
                ),
                Text(
                  'Changes: ${attendanceData.length}',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Colors.orange,
                        fontWeight: FontWeight.bold,
                      ),
                ),
              ],
            ),
        ],
      ),
    );
  }

  Widget _buildAttendanceList() {
    final studentState = ref.watch(studentProvider);
    final attendanceState = ref.watch(attendanceProvider);

    if (selectedClassId == null) {
      return const Center(
        child: Text('Please select a class to start tracking attendance'),
      );
    }

    if (studentState.students.isEmpty) {
      return const Center(
        child: Text('No students found in this class'),
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: studentState.students.length,
      itemBuilder: (context, index) {
        final student = studentState.students[index];
        final existingRecord = attendanceState.records.firstWhere(
          (record) => record.studentId == student.id!,
          orElse: () => AttendanceRecord(
            studentId: student.id!,
            classId: selectedClassId!,
            date: selectedDate,
            status: AttendanceStatus.absent,
          ),
        );

        final currentStatus =
            attendanceData[student.id] ?? existingRecord.status;
        final hasChanges = attendanceData.containsKey(student.id);

        return Card(
          margin: const EdgeInsets.only(bottom: 8),
          elevation: hasChanges ? 4 : 1,
          color: hasChanges ? Colors.orange.withValues(alpha: 0.1) : null,
          child: ListTile(
            leading: CircleAvatar(
              child: Text(student.displayName.substring(0, 1).toUpperCase()),
            ),
            title: Text(student.displayName),
            subtitle: Text('ID: ${student.studentId}'),
            trailing: _buildStatusDropdown(student.id!, currentStatus),
          ),
        );
      },
    );
  }

  Widget _buildStatusDropdown(int studentId, AttendanceStatus currentStatus) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        border: Border.all(color: Colors.grey),
        borderRadius: BorderRadius.circular(20),
        color: _getStatusColor(currentStatus).withValues(alpha: 0.2),
      ),
      child: DropdownButton<AttendanceStatus>(
        value: currentStatus,
        underline: const SizedBox(),
        isDense: true,
        items: AttendanceStatus.values.map((status) {
          return DropdownMenuItem<AttendanceStatus>(
            value: status,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                _buildStatusIcon(status),
                const SizedBox(width: 4),
                Text(
                  status.name.toUpperCase(),
                  style: TextStyle(
                    fontSize: 12,
                    color: _getStatusColor(status),
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
          );
        }).toList(),
        onChanged: (status) {
          if (status != null) {
            _onAttendanceStatusChanged(studentId, status);
          }
        },
      ),
    );
  }

  Widget _buildStatusIcon(AttendanceStatus status) {
    IconData icon;
    Color color;

    switch (status) {
      case AttendanceStatus.present:
        icon = Icons.check_circle;
        color = Colors.green;
        break;
      case AttendanceStatus.absent:
        icon = Icons.cancel;
        color = Colors.red;
        break;
      case AttendanceStatus.late:
        icon = Icons.access_time;
        color = Colors.orange;
        break;
      case AttendanceStatus.excused:
        icon = Icons.info;
        color = Colors.blue;
        break;
    }

    return Icon(icon, size: 16, color: color);
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
}
