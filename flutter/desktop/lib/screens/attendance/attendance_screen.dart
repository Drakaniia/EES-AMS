import 'package:flutter/material.dart';
import '../../widgets/layout/app_layout.dart';
import '../../themes/app_theme.dart';
import '../../widgets/common/loading_widget.dart';

class AttendanceScreen extends StatefulWidget {
  final int? classId;
  final DateTime? date;

  const AttendanceScreen({
    super.key,
    this.classId,
    this.date,
  });

  @override
  State<AttendanceScreen> createState() => _AttendanceScreenState();
}

class _AttendanceScreenState extends State<AttendanceScreen> {
  @override
  Widget build(BuildContext context) {
    return AppLayout(
      title: 'Attendance',
      showSidebar: true,
      child: const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.how_to_reg_outlined,
              size: 64,
              color: AppColors.primary,
            ),
            SizedBox(height: AppSpaces.md),
            Text(
              'Attendance Module',
              style: AppTextStyles.h3,
            ),
            SizedBox(height: AppSpaces.sm),
            Text(
              'Attendance tracking features will be implemented here',
              style: AppTextStyles.bodyMedium,
            ),
            SizedBox(height: AppSpaces.lg),
            LoadingWidget(message: 'Loading attendance data...'),
          ],
        ),
      ),
    );
  }
}