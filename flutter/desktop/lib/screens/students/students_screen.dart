import 'package:flutter/material.dart';
import '../../widgets/layout/app_layout.dart';
import '../../themes/app_theme.dart';
import '../../widgets/common/loading_widget.dart';

class StudentsScreen extends StatefulWidget {
  final int? filterClassId;
  final int? selectedStudentId;
  final bool isAddingNew;

  const StudentsScreen({
    super.key,
    this.filterClassId,
    this.selectedStudentId,
    this.isAddingNew = false,
  });

  @override
  State<StudentsScreen> createState() => _StudentsScreenState();
}

class _StudentsScreenState extends State<StudentsScreen> {
  @override
  Widget build(BuildContext context) {
    return AppLayout(
      title: 'Students',
      showSidebar: true,
      child: const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.people_outline,
              size: 64,
              color: AppColors.primary,
            ),
            SizedBox(height: AppSpaces.md),
            Text(
              'Students Module',
              style: AppTextStyles.h3,
            ),
            SizedBox(height: AppSpaces.sm),
            Text(
              'Student management features will be implemented here',
              style: AppTextStyles.bodyMedium,
            ),
            SizedBox(height: AppSpaces.lg),
            LoadingWidget(message: 'Loading student data...'),
          ],
        ),
      ),
    );
  }
}