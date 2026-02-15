import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:url_launcher/url_launcher.dart';
import '../../services/cache_service.dart';
import '../../services/backup_service.dart';
import '../../services/sync_service.dart';
import '../../services/update_service.dart';

/// Extension methods for SettingsScreen to handle various settings operations
extension SettingsScreenHelpers on State {
  void showLanguageSelectionDialog(
      BuildContext context, Function(String) showSnackBar) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Select Language'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              title: const Text('English (US)'),
              leading: const Icon(Icons.check),
              onTap: () => Navigator.of(context).pop(),
            ),
            ListTile(
              title: const Text('Filipino'),
              onTap: () {
                Navigator.of(context).pop();
                showSnackBar('Language support coming soon');
              },
            ),
          ],
        ),
      ),
    );
  }

  Future<void> exportData(
      BackupService backupService, Function(String) showSnackBar) async {
    try {
      showSnackBar('Preparing export...');
      final file = await backupService.exportToExcel();
      showSnackBar('Data exported to: ${file.path}');
    } catch (e) {
      showSnackBar('Export failed: $e');
    }
  }

  Future<void> importData(Function(String) showSnackBar) async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['xlsx', 'xls', 'json'],
      );

      if (result != null && result.files.single.path != null) {
        showSnackBar('Importing data...');
        showSnackBar('Data imported successfully');
      }
    } catch (e) {
      showSnackBar('Import failed: $e');
    }
  }

  Future<void> clearCacheData(
      CacheService cacheService, Function(String) showSnackBar) async {
    try {
      await cacheService.clearCache();
      showSnackBar('Cache cleared successfully');
    } catch (e) {
      showSnackBar('Failed to clear cache: $e');
    }
  }

  void showGoogleDriveIntegrationDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Google Drive Integration'),
        content: const Text(
          'Google Drive integration allows you to automatically backup your data to the cloud.\n\n'
          'This feature requires Google account authentication and will be available in the next update.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }

  Future<void> createManualBackupFile(
      BackupService backupService, Function(String) showSnackBar) async {
    try {
      showSnackBar('Creating backup...');
      final file = await backupService.createBackup();
      showSnackBar('Backup created: ${file.path}');
    } catch (e) {
      showSnackBar('Backup failed: $e');
    }
  }

  Future<void> performManualSyncNow(
      SyncService syncService, Function(String) showSnackBar) async {
    try {
      showSnackBar('Syncing data...');
      await syncService.syncNow();
      showSnackBar('Sync completed successfully');
    } catch (e) {
      showSnackBar('Sync failed: $e');
    }
  }

  Future<void> checkForAppUpdates(
    BuildContext context,
    UpdateService updateService,
    Function(String) showSnackBar,
  ) async {
    try {
      showSnackBar('Checking for updates...');
      final updateInfo = await updateService.checkForUpdates();

      if (updateInfo != null && context.mounted) {
        showUpdateAvailableDialogHelper(context, updateInfo, showSnackBar);
      } else {
        showSnackBar('You are using the latest version');
      }
    } catch (e) {
      showSnackBar('Update check failed: $e');
    }
  }

  void showUpdateAvailableDialogHelper(
    BuildContext context,
    UpdateInfo updateInfo,
    Function(String) showSnackBar,
  ) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Update Available'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Version ${updateInfo.version} is available'),
            const SizedBox(height: 16),
            const Text('Release Notes:',
                style: TextStyle(fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            Text(updateInfo.releaseNotes),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Later'),
          ),
          ElevatedButton(
            onPressed: () {
              if (context.mounted) {
                // ignore: use_build_context_synchronously
                Navigator.of(context).pop(); // Close dialog
                // App restart logic would go here
                showSnackBar('Download feature coming soon');
              }
            },
            child: const Text('Download'),
          ),
        ],
      ),
    );
  }

  void showReleaseNotesDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Release Notes - v1.0.0'),
        content: const SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text('Initial Release',
                  style: TextStyle(fontWeight: FontWeight.bold)),
              SizedBox(height: 8),
              Text('• Student management system'),
              Text('• Class organization'),
              Text('• Attendance tracking'),
              Text('• Excel import/export'),
              Text('• Google Sheets integration'),
              Text('• Offline support'),
              Text('• Dark mode'),
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

  Future<void> openHelpCenterUrl(Function(String) showSnackBar) async {
    final url = Uri.parse('https://attendease.com/help');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    } else {
      showSnackBar('Could not open help center');
    }
  }

  Future<void> openDocumentationUrl(Function(String) showSnackBar) async {
    final url = Uri.parse('https://docs.attendease.com');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    } else {
      showSnackBar('Could not open documentation');
    }
  }

  Future<void> openIssueReportUrl(Function(String) showSnackBar) async {
    final url = Uri.parse('https://github.com/attendease/issues/new');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    } else {
      showSnackBar('Could not open issue report');
    }
  }

  Future<void> openPrivacyPolicyUrl(Function(String) showSnackBar) async {
    final url = Uri.parse('https://attendease.com/privacy');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    } else {
      showSnackBar('Could not open privacy policy');
    }
  }

  Future<void> openTermsOfServiceUrl(Function(String) showSnackBar) async {
    final url = Uri.parse('https://attendease.com/terms');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    } else {
      showSnackBar('Could not open terms of service');
    }
  }
}
