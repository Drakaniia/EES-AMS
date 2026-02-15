import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../services/backup_service.dart';

final backupServiceProvider = Provider<BackupService>((ref) {
  return BackupService();
});
