import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/class.dart';
import '../../models/attendance.dart';
import '../../providers/class_provider.dart';
import '../../providers/student_provider.dart';
import '../../providers/attendance_provider.dart';
import '../../widgets/common/loading_widget.dart';
import '../../widgets/common/error_widget.dart';

class ClassManagementScreen extends ConsumerStatefulWidget {
  const ClassManagementScreen({super.key});

  @override
  ConsumerState<ClassManagementScreen> createState() =>
      _ClassManagementScreenState();
}

class _ClassManagementScreenState extends ConsumerState<ClassManagementScreen> {
  @override
  void initState() {
    super.initState();
    _loadClasses();
  }

  Future<void> _loadClasses() async {
    await ref.read(classProvider.notifier).loadClasses();
  }

  void _showAddClassDialog() {
    _showClassDialog();
  }

  void _showEditClassDialog(Class class_) {
    _showClassDialog(class_: class_);
  }

  void _showClassDialog({Class? class_}) {
    final isEditing = class_ != null;
    final nameController = TextEditingController(text: class_?.name ?? '');
    final sectionController =
        TextEditingController(text: class_?.section ?? '');
    final schoolYearController =
        TextEditingController(text: class_?.schoolYear ?? '');

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(isEditing ? 'Edit Class' : 'Add New Class'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextFormField(
                controller: nameController,
                decoration: const InputDecoration(
                  labelText: 'Class Name *',
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.class_),
                ),
                textCapitalization: TextCapitalization.words,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: sectionController,
                decoration: const InputDecoration(
                  labelText: 'Section',
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.category),
                ),
                textCapitalization: TextCapitalization.characters,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: schoolYearController,
                decoration: const InputDecoration(
                  labelText: 'School Year',
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.calendar_today),
                  helperText: 'Example: 2024-2025',
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              if (nameController.text.trim().isEmpty) {
                _showSnackBar('Class name is required', isError: true);
                return;
              }

              final classData = {
                'name': nameController.text.trim(),
                if (sectionController.text.trim().isNotEmpty)
                  'section': sectionController.text.trim(),
                if (schoolYearController.text.trim().isNotEmpty)
                  'school_year': schoolYearController.text.trim(),
              };

              Navigator.of(context).pop();

              if (isEditing && class_.id != null) {
                await ref
                    .read(classProvider.notifier)
                    .updateClass(class_.id!, classData);
              } else {
                await ref.read(classProvider.notifier).createClass(classData);
              }

              _handleOperationResult(isEditing
                  ? 'Class updated successfully'
                  : 'Class created successfully');
            },
            child: Text(isEditing ? 'Update' : 'Create'),
          ),
        ],
      ),
    );
  }

  void _showDeleteConfirmation(Class class_) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Class'),
        content: Text(
            'Are you sure you want to delete "${class_.displayName}"? This action cannot be undone.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.of(context).pop();
              if (class_.id != null) {
                ref.read(classProvider.notifier).deleteClass(class_.id!);
                _handleOperationResult('Class deleted successfully');
              }
            },
            style: ElevatedButton.styleFrom(backgroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
  }

  void _showClassDetails(Class class_) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (context) => ClassDetailsSheet(class_: class_),
    );
  }

  void _showStudentsCount(int classId) {
    Navigator.of(context)
        .pushNamed('/students', arguments: {'classId': classId});
  }

  void _handleOperationResult(String successMessage) {
    final state = ref.read(classProvider);
    if (state.error != null) {
      _showSnackBar(state.error!, isError: true);
    } else {
      _showSnackBar(successMessage);
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

    return Scaffold(
      appBar: AppBar(
        title: const Text('Class Management'),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
        actions: [
          IconButton(
            onPressed: _showAddClassDialog,
            icon: const Icon(Icons.add),
            tooltip: 'Add New Class',
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _loadClasses,
        child: classState.isLoading
            ? const LoadingWidget()
            : classState.error != null
                ? CommonErrorWidget(
                    error: classState.error!,
                    onRetry: _loadClasses,
                  )
                : classState.classes.isEmpty
                    ? _buildEmptyState()
                    : _buildClassesList(classState.classes),
      ),
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.class_outlined,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            'No classes found',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          Text(
            'Get started by adding your first class',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: _showAddClassDialog,
            icon: const Icon(Icons.add),
            label: const Text('Add Class'),
          ),
        ],
      ),
    );
  }

  Widget _buildClassesList(List<Class> classes) {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: classes.length,
      itemBuilder: (context, index) {
        final class_ = classes[index];
        return _buildClassCard(class_);
      },
    );
  }

  Widget _buildClassCard(Class class_) {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      elevation: 4,
      child: InkWell(
        onTap: () => _showClassDetails(class_),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  CircleAvatar(
                    backgroundColor:
                        Theme.of(context).colorScheme.primary.withValues(alpha: 0.1),
                    child: Text(
                      class_.name.substring(0, 1).toUpperCase(),
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.primary,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          class_.name,
                          style:
                              Theme.of(context).textTheme.titleLarge?.copyWith(
                                    fontWeight: FontWeight.bold,
                                  ),
                        ),
                        if (class_.section != null) ...[
                          const SizedBox(height: 4),
                          Text(
                            'Section: ${class_.section}',
                            style: Theme.of(context)
                                .textTheme
                                .bodyMedium
                                ?.copyWith(
                                  color: Theme.of(context)
                                      .colorScheme
                                      .onSurfaceVariant,
                                ),
                          ),
                        ],
                        if (class_.schoolYear != null) ...[
                          const SizedBox(height: 2),
                          Text(
                            'Year: ${class_.schoolYear}',
                            style:
                                Theme.of(context).textTheme.bodySmall?.copyWith(
                                      color: Theme.of(context)
                                          .colorScheme
                                          .onSurfaceVariant,
                                    ),
                          ),
                        ],
                      ],
                    ),
                  ),
                  PopupMenuButton<String>(
                    onSelected: (value) {
                      switch (value) {
                        case 'edit':
                          _showEditClassDialog(class_);
                          break;
                        case 'delete':
                          _showDeleteConfirmation(class_);
                          break;
                        case 'students':
                          if (class_.id != null) {
                            _showStudentsCount(class_.id!);
                          }
                          break;
                      }
                    },
                    itemBuilder: (context) => [
                      const PopupMenuItem(
                        value: 'students',
                        child: Row(
                          children: [
                            Icon(Icons.people, size: 18),
                            SizedBox(width: 8),
                            Text('View Students'),
                          ],
                        ),
                      ),
                      const PopupMenuItem(
                        value: 'edit',
                        child: Row(
                          children: [
                            Icon(Icons.edit, size: 18),
                            SizedBox(width: 8),
                            Text('Edit'),
                          ],
                        ),
                      ),
                      const PopupMenuItem(
                        value: 'delete',
                        child: Row(
                          children: [
                            Icon(Icons.delete, size: 18, color: Colors.red),
                            SizedBox(width: 8),
                            Text('Delete', style: TextStyle(color: Colors.red)),
                          ],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 16),
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceAround,
                children: [
                  FutureBuilder<int>(
                    future: _getStudentCount(class_.id!),
                    builder: (context, snapshot) {
                      return _buildStatItem(
                        icon: Icons.people,
                        label: 'Students',
                        value: '${snapshot.data ?? 0}',
                        onTap: () {
                          if (class_.id != null) {
                            _showStudentsCount(class_.id!);
                          }
                        },
                      );
                    },
                  ),
                  FutureBuilder<int>(
                    future: _getTodayAttendance(class_.id!),
                    builder: (context, snapshot) {
                      return _buildStatItem(
                        icon: Icons.calendar_today,
                        label: 'Today',
                        value: '${snapshot.data ?? 0}',
                        onTap: () {
                          Navigator.of(context).pushNamed(
                            '/attendance',
                            arguments: {'classId': class_.id},
                          );
                        },
                      );
                    },
                  ),
                  FutureBuilder<String>(
                    future: _getAttendanceRate(class_.id!),
                    builder: (context, snapshot) {
                      return _buildStatItem(
                        icon: Icons.assessment,
                        label: 'Rate',
                        value: '${snapshot.data ?? '0'}%',
                        onTap: () {
                          _showDetailedStats(class_.id!);
                        },
                      );
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStatItem({
    required IconData icon,
    required String label,
    required String value,
    VoidCallback? onTap,
  }) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: Column(
          children: [
            Icon(
              icon,
              size: 24,
              color: Theme.of(context).colorScheme.primary,
            ),
            const SizedBox(height: 4),
            Text(
              value,
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
            ),
            Text(
              label,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
          ],
        ),
      ),
    );
  }

  Future<int> _getStudentCount(int classId) async {
    try {
      // Load students for the specific class
      await ref.read(studentProvider.notifier).loadStudents(classId: classId);
      final studentState = ref.read(studentProvider);
      final students = studentState.students.where((s) => s.classId == classId);
      return students.length;
    } catch (e) {
      return 0;
    }
  }

  Future<int> _getTodayAttendance(int classId) async {
    try {
      // Load attendance for today
      final today = DateTime.now();
      await ref.read(attendanceProvider.notifier).loadAttendance(
            classId: classId,
            date: today,
          );
      final attendanceState = ref.read(attendanceProvider);
      
      // Filter records for today's date and specific class
      final todayRecords = attendanceState.records.where((record) {
        return record.classId == classId &&
            record.date.year == today.year &&
            record.date.month == today.month &&
            record.date.day == today.day;
      });
      
      return todayRecords.length;
    } catch (e) {
      return 0;
    }
  }

  Future<String> _getAttendanceRate(int classId) async {
    try {
      // Load both students and attendance data
      await ref.read(studentProvider.notifier).loadStudents(classId: classId);
      final studentState = ref.read(studentProvider);
      final students = studentState.students.where((s) => s.classId == classId);
      
      if (students.isEmpty) return '0';

      final today = DateTime.now();
      await ref.read(attendanceProvider.notifier).loadAttendance(
            classId: classId,
            date: today,
          );
      final attendanceState = ref.read(attendanceProvider);
      
      // Filter records for today's date and specific class
      final todayRecords = attendanceState.records.where((record) {
        return record.classId == classId &&
            record.date.year == today.year &&
            record.date.month == today.month &&
            record.date.day == today.day;
      });

      final presentCount = todayRecords
          .where((record) =>
              record.status == AttendanceStatus.present ||
              record.status == AttendanceStatus.late)
          .length;

      final rate = ((presentCount / students.length) * 100).round();
      return rate.toString();
    } catch (e) {
      return '0';
    }
  }

  void _showDetailedStats(int classId) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Class Statistics'),
        content: SizedBox(
          width: 400,
          height: 300,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              FutureBuilder<Map<String, dynamic>>(
                future: _getClassStatistics(classId, ref),
                builder: (context, snapshot) {
                  if (snapshot.hasData) {
                    final stats = snapshot.data!;
                    return Expanded(
                      child: SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildStatCard('Total Students', stats['totalStudents'].toString()),
                            const SizedBox(height: 12),
                            _buildStatCard('Today\'s Attendance', '${stats['todayAttendance']} (${stats['attendanceRate']}%)'),
                            const SizedBox(height: 12),
                            _buildStatCard('Weekly Attendance Rate', '${stats['weeklyAttendanceRate']}%'),
                            const SizedBox(height: 12),
                            _buildStatCard('Monthly Attendance Rate', '${stats['monthlyAttendanceRate']}%'),
                            const SizedBox(height: 12),
                            _buildAttendanceBreakdown('Attendance Breakdown', stats['attendanceBreakdown']),
                            const SizedBox(height: 12),
                            _buildTopPerformers('Top Attenders', stats['topAttenders'], context),
                          ],
                        ),
                      ),
                    );
                  } else {
                    return const Center(child: CircularProgressIndicator());
                  }
                },
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
  Future<Map<String, dynamic>> _getClassStatistics(int classId, WidgetRef ref) async {
    try {
      // Load students for the class
      await ref.read(studentProvider.notifier).loadStudents(classId: classId);
      final studentState = ref.read(studentProvider);
      final students = studentState.students.where((s) => s.classId == classId).toList();
      
      // Load attendance for the class
      await ref.read(attendanceProvider.notifier).loadAttendance(classId: classId);
      final attendanceState = ref.read(attendanceProvider);
      final classAttendance = attendanceState.records.where((a) => a.classId == classId).toList();
      
      // Calculate today's attendance
      final today = DateTime.now();
      final todayRecords = classAttendance.where((record) {
        return record.date.year == today.year &&
            record.date.month == today.month &&
            record.date.day == today.day;
      }).toList();
      
      // Calculate attendance rate for today
      int presentToday = 0;
      if (students.isNotEmpty) {
        presentToday = todayRecords.where((record) =>
            record.status == AttendanceStatus.present ||
            record.status == AttendanceStatus.late).length;
      }
      final todayRate = students.isNotEmpty ? ((presentToday / students.length) * 100).round() : 0;
      
      // Calculate weekly attendance (last 7 days)
      final weekAgo = DateTime.now().subtract(const Duration(days: 7));
      final weeklyRecords = classAttendance.where((record) {
        return record.date.isAfter(weekAgo) || record.date.isAtSameMomentAs(weekAgo);
      }).toList();
      
      // Group weekly records by student
      final Map<int, List<AttendanceRecord>> weeklyByStudent = {};
      for (final record in weeklyRecords) {
        if (!weeklyByStudent.containsKey(record.studentId)) {
          weeklyByStudent[record.studentId] = [];
        }
        weeklyByStudent[record.studentId]!.add(record);
      }
      
      // Calculate weekly attendance rate
      int totalWeeklyPossible = students.length * 7; // Assuming 7 days of possible attendance
      int totalWeeklyPresent = 0;
      
      for (final student in students) {
        final studentWeeklyRecords = weeklyByStudent[student.id] ?? [];
        for (final record in studentWeeklyRecords) {
          if (record.status == AttendanceStatus.present || record.status == AttendanceStatus.late) {
            totalWeeklyPresent++;
          }
        }
      }
      
      final weeklyRate = totalWeeklyPossible > 0 ? ((totalWeeklyPresent / totalWeeklyPossible) * 100).round() : 0;
      
      // Calculate monthly attendance (last 30 days)
      final monthAgo = DateTime.now().subtract(const Duration(days: 30));
      final monthlyRecords = classAttendance.where((record) {
        return record.date.isAfter(monthAgo) || record.date.isAtSameMomentAs(monthAgo);
      }).toList();
      
      // Group monthly records by student
      final Map<int, List<AttendanceRecord>> monthlyByStudent = {};
      for (final record in monthlyRecords) {
        if (!monthlyByStudent.containsKey(record.studentId)) {
          monthlyByStudent[record.studentId] = [];
        }
        monthlyByStudent[record.studentId]!.add(record);
      }
      
      // Calculate monthly attendance rate
      int totalMonthlyPossible = students.length * 30; // Assuming 30 days of possible attendance
      int totalMonthlyPresent = 0;
      
      for (final student in students) {
        final studentMonthlyRecords = monthlyByStudent[student.id] ?? [];
        for (final record in studentMonthlyRecords) {
          if (record.status == AttendanceStatus.present || record.status == AttendanceStatus.late) {
            totalMonthlyPresent++;
          }
        }
      }
      
      final monthlyRate = totalMonthlyPossible > 0 ? ((totalMonthlyPresent / totalMonthlyPossible) * 100).round() : 0;
      
      // Calculate attendance breakdown
      final Map<String, int> attendanceBreakdown = {
        'Present': todayRecords.where((r) => r.status == AttendanceStatus.present).length,
        'Late': todayRecords.where((r) => r.status == AttendanceStatus.late).length,
        'Absent': todayRecords.where((r) => r.status == AttendanceStatus.absent).length,
        'Excused': todayRecords.where((r) => r.status == AttendanceStatus.excused).length,
      };
      
      // Calculate top attenders (students with best attendance rates in the last 30 days)
      final List<Map<String, dynamic>> topAttenders = [];
      for (final student in students) {
        final studentMonthlyRecords = monthlyByStudent[student.id] ?? [];
        if (studentMonthlyRecords.isNotEmpty) {
          final presentCount = studentMonthlyRecords.where((r) => 
            r.status == AttendanceStatus.present || r.status == AttendanceStatus.late
          ).length;
          final attendanceRate = (presentCount / studentMonthlyRecords.length) * 100;
          
          topAttenders.add({
            'name': '${student.firstName} ${student.lastName}',
            'rate': attendanceRate.round(),
            'present': presentCount,
            'total': studentMonthlyRecords.length,
          });
        }
      }
      
      // Sort top attenders by rate (descending)
      topAttenders.sort((a, b) => b['rate'].compareTo(a['rate']));
      
      return {
        'totalStudents': students.length,
        'todayAttendance': presentToday,
        'attendanceRate': todayRate,
        'weeklyAttendanceRate': weeklyRate,
        'monthlyAttendanceRate': monthlyRate,
        'attendanceBreakdown': attendanceBreakdown,
        'topAttenders': topAttenders.take(5).toList(), // Top 5
      };
    } catch (e) {
      // Return default values in case of error
      return {
        'totalStudents': 0,
        'todayAttendance': 0,
        'attendanceRate': 0,
        'weeklyAttendanceRate': 0,
        'monthlyAttendanceRate': 0,
        'attendanceBreakdown': {'Present': 0, 'Late': 0, 'Absent': 0, 'Excused': 0},
        'topAttenders': [],
      };
    }
  }
  
  Widget _buildStatCard(String title, String value) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.grey[50],
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Colors.grey[300]!),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            title,
            style: const TextStyle(
              fontSize: 14,
              fontWeight: FontWeight.w500,
              color: Colors.grey,
            ),
          ),
          Text(
            value,
            style: const TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              ),
          ),
        ],
      ),
    );
  }
  
  Widget _buildAttendanceBreakdown(String title, Map<String, int> breakdown) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        ...breakdown.entries.map((entry) {
          final percentage = breakdown.values.reduce((a, b) => a + b) > 0
              ? ((entry.value / breakdown.values.reduce((a, b) => a + b)) * 100).round()
              : 0;

          return Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Row(
              children: [
                Container(
                  width: 12,
                  height: 12,
                  decoration: BoxDecoration(
                    color: _getStatusColor(entry.key),
                    shape: BoxShape.circle,
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  flex: 2,
                  child: Text(entry.key),
                ),
                Expanded(
                  child: Text('${entry.value} ($percentage%)'),
                ),
              ],
            ),
          );
        }),
      ],
    );
  }
  
  Color _getStatusColor(String status) {
    switch (status) {
      case 'Present':
        return Colors.green;
      case 'Late':
        return Colors.orange;
      case 'Absent':
        return Colors.red;
      case 'Excused':
        return Colors.blue;
      default:
        return Colors.grey;
    }
  }
  
  Widget _buildTopPerformers(String title, List<dynamic> topAttenders, BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 8),
        if (topAttenders.isEmpty)
          const Text('No attendance data available', style: TextStyle(color: Colors.grey))
        else
          ...topAttenders.asMap().entries.map((entry) {
            final index = entry.key;
            final attender = entry.value;
            return Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Row(
                children: [
                  Container(
                    width: 24,
                    height: 24,
                    decoration: BoxDecoration(
                      color: Theme.of(context).primaryColor,
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Center(
                      child: Text(
                        '${index + 1}',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    flex: 3,
                    child: Text(
                      attender['name'],
                      style: const TextStyle(fontWeight: FontWeight.w500),
                    ),
                  ),
                  Expanded(
                    child: Text('${attender['rate']}%'),
                  ),
                ],
              ),
            );
          }),
      ],
    );
  }
}

class ClassDetailsSheet extends ConsumerWidget {
  final Class class_;

  const ClassDetailsSheet({
    super.key,
    required this.class_,
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
                'Class Details',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              IconButton(
                onPressed: () => Navigator.of(context).pop(),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
          const SizedBox(height: 16),
          _buildDetailRow('Name', class_.name),
          if (class_.section != null)
            _buildDetailRow('Section', class_.section!),
          if (class_.schoolYear != null)
            _buildDetailRow('School Year', class_.schoolYear!),
          if (class_.id != null)
            _buildDetailRow('Class ID', class_.id!.toString()),
          if (class_.createdAt != null)
            _buildDetailRow('Created', _formatDate(class_.createdAt!)),
          if (class_.updatedAt != null)
            _buildDetailRow('Updated', _formatDate(class_.updatedAt!)),
          const SizedBox(height: 24),
          Row(
            children: [
              Expanded(
                child: ElevatedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                    if (class_.id != null) {
                      Navigator.of(context).pushNamed('/students',
                          arguments: {'classId': class_.id});
                    }
                  },
                  icon: const Icon(Icons.people),
                  label: const Text('Manage Students'),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: ElevatedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                    Navigator.of(context).pushNamed(
                      '/attendance',
                      arguments: {'classId': class_.id},
                    );
                  },
                  icon: const Icon(Icons.assignment),
                  label: const Text('Take Attendance'),
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
          Expanded(child: Text(value)),
        ],
      ),
    );
  }

  String _formatDate(DateTime date) {
    return '${date.day.toString().padLeft(2, '0')}-${date.month.toString().padLeft(2, '0')}-${date.year}';
  }
}
