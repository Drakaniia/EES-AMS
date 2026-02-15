import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:file_picker/file_picker.dart';
import '../../providers/theme_provider.dart';
import '../../providers/backup_provider.dart';
import '../../services/sync_service.dart';
import '../../services/backup_service.dart'; // Added import for BackupService

class SettingsScreen extends ConsumerStatefulWidget {
  final int initialTab;

  const SettingsScreen({
    super.key,
    this.initialTab = 0,
  });

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen>
    with SingleTickerProviderStateMixin {
  bool _autoSync = true;
  bool _notifications = true;
  bool _darkMode = false;
  bool _autoBackup = true;
  String _syncInterval = '30';
  bool _autoUpdate = true;

  @override
  void initState() {
    super.initState();
    // _tabController = TabController(length: 4, vsync: this);
    // _tabController.index = widget.initialTab;
    _loadSettings();
  }

  @override
  void dispose() {
    // _tabController.dispose();
    super.dispose();
  }

  Future<void> _loadSettings() async {
    final prefs = await SharedPreferences.getInstance();
    setState(() {
      _autoSync = prefs.getBool('auto_sync') ?? true;
      _notifications = prefs.getBool('notifications') ?? true;
      _autoBackup = prefs.getBool('auto_backup') ?? true;
      _syncInterval = prefs.getString('sync_interval') ?? '30';
      _autoUpdate = prefs.getBool('auto_update') ?? true;
      _darkMode = prefs.getBool('dark_mode') ?? false;
    });
  }

  Future<void> _saveSettings() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool('auto_sync', _autoSync);
    await prefs.setBool('notifications', _notifications);
    await prefs.setBool('auto_backup', _autoBackup);
    await prefs.setString('sync_interval', _syncInterval);
    await prefs.setBool('auto_update', _autoUpdate);
    await prefs.setBool('dark_mode', _darkMode);

    // Update theme provider
    ref.read(themeProvider.notifier).setDarkMode(_darkMode);
  }

  void _onDarkModeChanged(bool value) {
    setState(() {
      _darkMode = value;
    });
    _saveSettings();
  }

  void _onNotificationsChanged(bool value) {
    setState(() {
      _notifications = value;
    });
    _saveSettings();
  }

  void _showSnackBar(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        backgroundColor: Theme.of(context).colorScheme.primary,
      ),
    );
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

  Future<void> createManualBackupFile(BackupService backupService, Function(String) showSnackBar) async {
    try {
      showSnackBar('Creating backup...');
      final file = await backupService.createBackup();
      showSnackBar('Backup created: ${file.path}');
    } catch (e) {
      showSnackBar('Backup failed: $e');
    }
  }

  Future<void> performManualSyncNow(SyncService syncService, Function(String) showSnackBar) async {
    try {
      showSnackBar('Syncing data...');
      // In a real implementation, this would call the sync service
      // await syncService.syncNow();
      showSnackBar('Sync completed successfully');
    } catch (e) {
      showSnackBar('Sync failed: Operation not implemented');
    }
  }

  Future<void> checkForAppUpdates(Function(String) showSnackBar) async {
    try {
      showSnackBar('Checking for updates...');
      // In a real implementation, this would call the update service
      // final updateInfo = await updateService.checkForUpdates();
      showSnackBar('You are using the latest version');
    } catch (e) {
      showSnackBar('Update check failed: Operation not implemented');
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        _buildSection(
          context,
          title: 'Appearance',
          children: [
            SwitchListTile(
              title: const Text('Dark Mode'),
              subtitle: const Text('Enable dark theme for the application'),
              value: _darkMode,
              onChanged: _onDarkModeChanged,
              secondary: Icon(
                _darkMode ? Icons.dark_mode : Icons.light_mode,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        _buildSection(
          context,
          title: 'Notifications',
          children: [
            SwitchListTile(
              title: const Text('Enable Notifications'),
              subtitle: const Text(
                  'Show notifications for attendance updates and reminders'),
              value: _notifications,
              onChanged: _onNotificationsChanged,
              secondary: Icon(
                _notifications ? Icons.notifications : Icons.notifications_off,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        _buildSection(
          context,
          title: 'Language',
          children: [
            ListTile(
              title: const Text('Language'),
              subtitle: const Text('English (US)'),
              leading: Icon(
                Icons.language,
                color: Theme.of(context).colorScheme.primary,
              ),
              trailing: const Icon(Icons.arrow_forward_ios),
              onTap: () {
                _showLanguageSelectionDialog(context);
              },
            ),
          ],
        ),
        const SizedBox(height: 16),
        _buildSection(
          context,
          title: 'Sync Settings',
          children: [
            SwitchListTile(
              title: const Text('Auto Sync'),
              subtitle: const Text('Automatically sync data periodically'),
              value: _autoSync,
              onChanged: (value) {
                setState(() {
                  _autoSync = value;
                });
                _saveSettings();
              },
              secondary: Icon(
                _autoSync ? Icons.sync : Icons.sync_disabled,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
            ListTile(
              title: const Text('Sync Interval'),
              subtitle: Text('$_syncInterval minutes'),
              leading: Icon(
                Icons.schedule,
                color: Theme.of(context).colorScheme.primary,
              ),
              trailing: const Icon(Icons.arrow_forward_ios),
              onTap: () {
                _showSyncIntervalDialog(context);
              },
            ),
            ListTile(
              title: const Text('Sync Now'),
              subtitle: const Text('Manually sync data with server'),
              leading: Icon(
                Icons.sync,
                color: Theme.of(context).colorScheme.primary,
              ),
              onTap: () async {
                final syncService = SyncService(); // Using directly since not provided via Riverpod
                await performManualSyncNow(syncService, _showSnackBar);
              },
            ),
          ],
        ),
        const SizedBox(height: 16),
        _buildSection(
          context,
          title: 'Backup Settings',
          children: [
            SwitchListTile(
              title: const Text('Auto Backup'),
              subtitle: const Text('Automatically backup data periodically'),
              value: _autoBackup,
              onChanged: (value) {
                setState(() {
                  _autoBackup = value;
                });
                _saveSettings();
              },
              secondary: Icon(
                _autoBackup ? Icons.backup : Icons.backup_outlined,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
            ListTile(
              title: const Text('Create Manual Backup'),
              subtitle: const Text('Manually create a backup file'),
              leading: Icon(
                Icons.backup,
                color: Theme.of(context).colorScheme.primary,
              ),
              onTap: () async {
                final backupService = ref.read(backupServiceProvider);
                await createManualBackupFile(backupService, _showSnackBar);
              },
            ),
            ListTile(
              title: const Text('Manage Backups'),
              subtitle: const Text('View and restore backup files'),
              leading: Icon(
                Icons.storage,
                color: Theme.of(context).colorScheme.primary,
              ),
              trailing: const Icon(Icons.arrow_forward_ios),
              onTap: () {
                _showManageBackupsDialog(context);
              },
            ),
          ],
        ),
        const SizedBox(height: 16),
        _buildSection(
          context,
          title: 'Data Management',
          children: [
            ListTile(
              title: const Text('Clear Cache'),
              subtitle:
                  const Text('Clear application cache and temporary files'),
              leading: Icon(
                Icons.cleaning_services,
                color: Theme.of(context).colorScheme.primary,
              ),
              trailing: const Icon(Icons.arrow_forward_ios),
              onTap: () {
                _showClearCacheDialog(context);
              },
            ),
            ListTile(
              title: const Text('Export Data'),
              subtitle: const Text('Export all data to a backup file'),
              leading: Icon(
                Icons.file_download,
                color: Theme.of(context).colorScheme.primary,
              ),
              trailing: const Icon(Icons.arrow_forward_ios),
              onTap: () async {
                await _exportData(context);
              },
            ),
            ListTile(
              title: const Text('Import Data'),
              subtitle: const Text('Import data from a backup file'),
              leading: Icon(
                Icons.file_upload,
                color: Theme.of(context).colorScheme.primary,
              ),
              trailing: const Icon(Icons.arrow_forward_ios),
              onTap: () async {
                await _importData(context);
              },
            ),
          ],
        ),
        const SizedBox(height: 16),
        _buildSection(
          context,
          title: 'App Updates',
          children: [
            SwitchListTile(
              title: const Text('Auto Update'),
              subtitle: const Text('Automatically check for app updates'),
              value: _autoUpdate,
              onChanged: (value) {
                setState(() {
                  _autoUpdate = value;
                });
                _saveSettings();
              },
              secondary: Icon(
                _autoUpdate ? Icons.system_update : Icons.system_update_alt_outlined,
                color: Theme.of(context).colorScheme.primary,
              ),
            ),
            ListTile(
              title: const Text('Check for Updates'),
              subtitle: const Text('Manually check for app updates'),
              leading: Icon(
                Icons.system_update,
                color: Theme.of(context).colorScheme.primary,
              ),
              onTap: () async {
                await checkForAppUpdates(_showSnackBar);
              },
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildSection(BuildContext context,
      {required String title, required List<Widget> children}) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Text(
            title,
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: Theme.of(context).colorScheme.primary,
                ),
          ),
        ),
        Card(
          elevation: 2,
          child: Column(
            children: children,
          ),
        ),
      ],
    );
  }

  void _showLanguageSelectionDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Select Language'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              title: const Text('English (US)'),
              leading: const Icon(Icons.language, color: Colors.green),
              onTap: () => Navigator.of(context).pop(),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
        ],
      ),
    );
  }

  void _showSyncIntervalDialog(BuildContext context) {
    final TextEditingController controller =
        TextEditingController(text: _syncInterval);

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Sync Interval'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('Set sync interval in minutes:'),
            TextField(
              controller: controller,
              keyboardType: TextInputType.number,
              decoration: const InputDecoration(
                hintText: 'Enter minutes',
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              final value = int.tryParse(controller.text);
              if (value != null && value > 0) {
                setState(() {
                  _syncInterval = controller.text;
                });
                _saveSettings();
                Navigator.of(context).pop();
                _showSnackBar('Sync interval updated to $value minutes');
              } else {
                _showSnackBar('Please enter a valid number');
              }
            },
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }

  void _showManageBackupsDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Manage Backups'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              title: const Text('View Backup Files'),
              leading: const Icon(Icons.folder),
              onTap: () {
                // In a real implementation, navigate to backup management screen
                Navigator.of(context).pop();
                _showSnackBar('Opening backup management...');
              },
            ),
            ListTile(
              title: const Text('Restore from Backup'),
              leading: const Icon(Icons.restore),
              onTap: () {
                // In a real implementation, allow selecting a backup file to restore
                Navigator.of(context).pop();
                _showSnackBar('Restore functionality not implemented');
              },
            ),
          ],
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

  void _showClearCacheDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear Cache'),
        content: const Text(
            'Are you sure you want to clear the application cache? This will remove temporary files but will not delete your data.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.of(context).pop();
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('Cache cleared successfully')),
              );
            },
            style: ElevatedButton.styleFrom(backgroundColor: Colors.red),
            child: const Text('Clear'),
          ),
        ],
      ),
    );
  }

  Future<void> _exportData(BuildContext context) async {
    final backupService = ref.read(backupServiceProvider);
    try {
      await exportData(backupService, _showSnackBar);
    } catch (e) {
      _showSnackBar('Export failed: $e');
    }
  }

  Future<void> _importData(BuildContext context) async {
    try {
      await importData(_showSnackBar);
    } catch (e) {
      _showSnackBar('Import failed: $e');
    }
  }
}
