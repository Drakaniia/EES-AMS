import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';
import '../../models/student.dart';
import '../../providers/student_provider.dart';
import '../../providers/class_provider.dart';
import '../../widgets/common/loading_widget.dart';
import '../../widgets/common/error_widget.dart';

class StudentManagementScreen extends ConsumerStatefulWidget {
  const StudentManagementScreen({super.key});

  @override
  ConsumerState<StudentManagementScreen> createState() =>
      _StudentManagementScreenState();
}

class _StudentManagementScreenState
    extends ConsumerState<StudentManagementScreen> {
  int? selectedClassId;
  String searchQuery = '';
  final TextEditingController searchController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _loadInitialData();
  }

  Future<void> _loadInitialData() async {
    await ref.read(classProvider.notifier).loadClasses();
    await _loadStudents();
  }

  Future<void> _loadStudents() async {
    if (searchQuery.trim().isNotEmpty) {
      await ref.read(studentProvider.notifier).searchStudents(
            searchQuery.trim(),
            classId: selectedClassId,
          );
    } else {
      await ref
          .read(studentProvider.notifier)
          .loadStudents(classId: selectedClassId);
    }
  }

  void _onClassChanged(int? classId) {
    setState(() {
      selectedClassId = classId;
    });
    _loadStudents();
  }

  void _onSearchChanged(String query) {
    setState(() {
      searchQuery = query;
    });
    _debouncedSearch();
  }

  void _debouncedSearch() {
    Future.delayed(const Duration(milliseconds: 500), () {
      if (mounted) {
        _loadStudents();
      }
    });
  }

  void _showAddStudentDialog() {
    _showStudentDialog();
  }

  void _showEditStudentDialog(Student student) {
    _showStudentDialog(student: student);
  }

  void _showStudentDialog({Student? student}) {
    final isEditing = student != null;
    final studentIdController =
        TextEditingController(text: student?.studentId ?? '');
    final firstNameController =
        TextEditingController(text: student?.firstName ?? '');
    final lastNameController =
        TextEditingController(text: student?.lastName ?? '');
    final emailController = TextEditingController(text: student?.email ?? '');
    final phoneController = TextEditingController(text: student?.phone ?? '');

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(isEditing ? 'Edit Student' : 'Add New Student'),
        content: SizedBox(
          width: 400,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextFormField(
                  controller: studentIdController,
                  decoration: const InputDecoration(
                    labelText: 'Student ID *',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.badge),
                  ),
                  textCapitalization: TextCapitalization.characters,
                ),
                const SizedBox(height: 16),
                Row(
                  children: [
                    Expanded(
                      child: TextFormField(
                        controller: firstNameController,
                        decoration: const InputDecoration(
                          labelText: 'First Name *',
                          border: OutlineInputBorder(),
                          prefixIcon: Icon(Icons.person),
                        ),
                        textCapitalization: TextCapitalization.words,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: TextFormField(
                        controller: lastNameController,
                        decoration: const InputDecoration(
                          labelText: 'Last Name *',
                          border: OutlineInputBorder(),
                          prefixIcon: Icon(Icons.person),
                        ),
                        textCapitalization: TextCapitalization.words,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                DropdownButtonFormField<int>(
                  initialValue: selectedClassId,
                  decoration: const InputDecoration(
                    labelText: 'Assign to Class (Optional)',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.class_),
                  ),
                  items: ref.read(classProvider).classes.map((class_) {
                    return DropdownMenuItem<int>(
                      value: class_.id,
                      child: Text(class_.displayName),
                    );
                  }).toList(),
                  onChanged: (value) {
                    setState(() {
                      selectedClassId = value;
                    });
                  },
                ),
                const SizedBox(height: 16),
                TextFormField(
                  controller: emailController,
                  decoration: const InputDecoration(
                    labelText: 'Email',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.email),
                  ),
                  keyboardType: TextInputType.emailAddress,
                ),
                const SizedBox(height: 16),
                TextFormField(
                  controller: phoneController,
                  decoration: const InputDecoration(
                    labelText: 'Phone',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.phone),
                  ),
                  keyboardType: TextInputType.phone,
                ),
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              if (studentIdController.text.trim().isEmpty ||
                  firstNameController.text.trim().isEmpty ||
                  lastNameController.text.trim().isEmpty) {
                _showSnackBar(
                    'Student ID, First Name, and Last Name are required',
                    isError: true);
                return;
              }

              final studentData = {
                'student_id': studentIdController.text.trim(),
                'first_name': firstNameController.text.trim(),
                'last_name': lastNameController.text.trim(),
                if (emailController.text.trim().isNotEmpty)
                  'email': emailController.text.trim(),
                if (phoneController.text.trim().isNotEmpty)
                  'phone': phoneController.text.trim(),
                if (selectedClassId != null) 'class_id': selectedClassId,
              };

              Navigator.of(context).pop();

              if (isEditing && student.id != null) {
                await ref
                    .read(studentProvider.notifier)
                    .updateStudent(student.id!, studentData);
              } else {
                await ref
                    .read(studentProvider.notifier)
                    .createStudent(studentData);
              }

              _handleOperationResult(isEditing
                  ? 'Student updated successfully'
                  : 'Student created successfully');
            },
            child: Text(isEditing ? 'Update' : 'Create'),
          ),
        ],
      ),
    );
  }

  void _showDeleteConfirmation(Student student) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Student'),
        content: Text(
            'Are you sure you want to delete "${student.displayName}"? This action cannot be undone.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.of(context).pop();
              if (student.id != null) {
                ref.read(studentProvider.notifier).deleteStudent(student.id!);
                _handleOperationResult('Student deleted successfully');
              }
            },
            style: ElevatedButton.styleFrom(backgroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
  }

  void _showStudentDetails(Student student) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (context) => StudentDetailsSheet(student: student),
    );
  }

  void _viewAttendanceHistory(Student student) {
    if (student.id != null) {
      Navigator.of(context).pushNamed(
        '/attendance/list',
        arguments: {'studentId': student.id},
      );
    }
  }

  void _importFromExcel() {
    Navigator.of(context).pushNamed('/students/import');
  }

  void _handleOperationResult(String successMessage) {
    final state = ref.read(studentProvider);
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
    final studentState = ref.watch(studentProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Student Management'),
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
        actions: [
          IconButton(
            onPressed: _importFromExcel,
            icon: const Icon(Icons.file_upload),
            tooltip: 'Import from Excel',
          ),
        ],
      ),
      body: Column(
        children: [
          _buildFiltersAndSearch(),
          if (studentState.isLoading)
            const Expanded(child: LoadingWidget())
          else if (studentState.error != null)
            Expanded(
              child: CommonErrorWidget(
                error: studentState.error!,
                onRetry: _loadStudents,
              ),
            )
          else
            Expanded(
              child: studentState.students.isEmpty
                  ? _buildEmptyState()
                  : _buildStudentsList(studentState.students),
            ),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _showAddStudentDialog,
        backgroundColor: Theme.of(context).colorScheme.primary,
        foregroundColor: Colors.white,
        child: const Icon(Icons.add),
      ),
    );
  }

  Widget _buildFiltersAndSearch() {
    final studentState = ref.watch(studentProvider);

    return Container(
      padding: const EdgeInsets.all(16),
      color: Theme.of(context).colorScheme.surface,
      child: Column(
        children: [
          Row(
            children: [
              Expanded(
                child: TextFormField(
                  controller: searchController,
                  decoration: const InputDecoration(
                    labelText: 'Search Students',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.search),
                    suffixIcon: Icon(Icons.clear),
                  ),
                  onChanged: _onSearchChanged,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: DropdownButtonFormField<int>(
                  initialValue: selectedClassId,
                  decoration: const InputDecoration(
                    labelText: 'Filter by Class',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.filter_list),
                  ),
                  items: [
                    const DropdownMenuItem<int>(
                      value: null,
                      child: Text('All Classes'),
                    ),
                    ...ref.read(classProvider).classes.map((class_) {
                      return DropdownMenuItem<int>(
                        value: class_.id,
                        child: Text(class_.displayName),
                      );
                    }),
                  ],
                  onChanged: _onClassChanged,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                '${studentState.students.length} students found',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.primary,
                    ),
              ),
            ],
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
            Icons.person_outline,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            searchQuery.trim().isNotEmpty || selectedClassId != null
                ? 'No students found'
                : 'No students registered',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 8),
          Text(
            searchQuery.trim().isNotEmpty || selectedClassId != null
                ? 'Try adjusting your filters or search'
                : 'Get started by adding your first student',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
          if (searchQuery.trim().isEmpty && selectedClassId == null) ...[
            const SizedBox(height: 24),
            ElevatedButton.icon(
              onPressed: _showAddStudentDialog,
              icon: const Icon(Icons.add),
              label: const Text('Add Student'),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildStudentsList(List<Student> students) {
    return RefreshIndicator(
      onRefresh: _loadStudents,
      child: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: students.length,
        itemBuilder: (context, index) {
          final student = students[index];
          return _buildStudentCard(student);
        },
      ),
    );
  }

  Widget _buildStudentCard(Student student) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      elevation: 2,
      child: InkWell(
        onTap: () => _showStudentDetails(student),
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              CircleAvatar(
                backgroundColor: Theme.of(context)
                    .colorScheme
                    .primary
                    .withValues(alpha: 0.2),
                child: Text(
                  student.displayName.substring(0, 1).toUpperCase(),
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
                      student.displayName,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'ID: ${student.studentId}',
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                            color:
                                Theme.of(context).colorScheme.onSurfaceVariant,
                          ),
                    ),
                    if (student.email != null) ...[
                      const SizedBox(height: 2),
                      Text(
                        student.email!,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context)
                                  .colorScheme
                                  .onSurfaceVariant,
                            ),
                      ),
                    ],
                    if (student.phone != null) ...[
                      const SizedBox(height: 2),
                      Text(
                        student.phone!,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
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
                      _showEditStudentDialog(student);
                      break;
                    case 'delete':
                      _showDeleteConfirmation(student);
                      break;
                    case 'attendance':
                      _viewAttendanceHistory(student);
                      break;
                  }
                },
                itemBuilder: (context) => [
                  const PopupMenuItem(
                    value: 'attendance',
                    child: Row(
                      children: [
                        Icon(Icons.assignment, size: 18),
                        SizedBox(width: 8),
                        Text('Attendance'),
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
        ),
      ),
    );
  }
}

class StudentDetailsSheet extends StatelessWidget {
  final Student student;

  const StudentDetailsSheet({
    super.key,
    required this.student,
  });

  @override
  Widget build(BuildContext context) {
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
                'Student Details',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              IconButton(
                onPressed: () => Navigator.of(context).pop(),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
          const SizedBox(height: 16),
          _buildDetailRow('Student ID', student.studentId),
          _buildDetailRow('First Name', student.firstName),
          _buildDetailRow('Last Name', student.lastName),
          _buildDetailRow('Full Name', student.displayName),
          if (student.email != null) _buildDetailRow('Email', student.email!),
          if (student.phone != null) _buildDetailRow('Phone', student.phone!),
          if (student.classId != null)
            _buildDetailRow('Class ID', student.classId.toString()),
          if (student.id != null)
            _buildDetailRow('Database ID', student.id.toString()),
          if (student.createdAt != null)
            _buildDetailRow('Created',
                DateFormat('MMM dd, yyyy HH:mm').format(student.createdAt!)),
          if (student.updatedAt != null)
            _buildDetailRow('Updated',
                DateFormat('MMM dd, yyyy HH:mm').format(student.updatedAt!)),
          const SizedBox(height: 24),
          Row(
            children: [
              Expanded(
                child: ElevatedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                    Navigator.of(context).pushNamed(
                      '/attendance/list',
                      arguments: {'studentId': student.id},
                    );
                  },
                  icon: const Icon(Icons.assignment),
                  label: const Text('View Attendance'),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: ElevatedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                    // Simple edit student dialog
                    showDialog(
                      context: context,
                      builder: (dialogContext) => AlertDialog(
                        title: const Text('Edit Student'),
                        content: SizedBox(
                          width: 400,
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Text('Student ID: ${student.studentId}'),
                              const SizedBox(height: 8),
                              Text(
                                  'Name: ${student.firstName} ${student.lastName}'),
                              const SizedBox(height: 8),
                              Text('Email: ${student.email ?? 'N/A'}'),
                              const SizedBox(height: 8),
                              Text('Phone: ${student.phone ?? 'N/A'}'),
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
                              Navigator.of(dialogContext).pop();
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(
                                    content: Text('Edit feature coming soon!')),
                              );
                            },
                            child: const Text('Save'),
                          ),
                        ],
                      ),
                    );
                  },
                  icon: const Icon(Icons.edit),
                  label: const Text('Edit Student'),
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
            width: 120,
            child: Text(
              label,
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
}
