import 'package:flutter/material.dart';
import '../../widgets/layout/app_layout.dart';
import '../../themes/app_theme.dart';
import '../../widgets/common/loading_widget.dart';

class ClassesScreen extends StatefulWidget {
  final int? selectedClassId;
  final bool isAddingNew;

  const ClassesScreen({
    super.key,
    this.selectedClassId,
    this.isAddingNew = false,
  });

  @override
  State<ClassesScreen> createState() => _ClassesScreenState();
}

class _ClassesScreenState extends State<ClassesScreen> {
  @override
  Widget build(BuildContext context) {
    return AppLayout(
      title: 'Classes',
      showSidebar: true,
      child: const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.class_outlined,
              size: 64,
              color: AppColors.primary,
            ),
            SizedBox(height: AppSpaces.md),
            Text(
              'Classes Module',
              style: AppTextStyles.h3,
            ),
            SizedBox(height: AppSpaces.sm),
            Text(
              'Class management features will be implemented here',
              style: AppTextStyles.bodyMedium,
            ),
            SizedBox(height: AppSpaces.lg),
            LoadingWidget(message: 'Loading class data...'),
          ],
        ),
      ),
    );
  }
}