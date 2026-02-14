# AttendEase - Auto-Update Setup Guide

This guide explains how to configure the auto-update functionality for the AttendEase application.

## Overview

The AttendEase application uses Tauri's built-in updater plugin to provide automatic updates. When updates are available, users will be notified and can choose to install them immediately or later.

## Requirements

1. **Tauri Updater Plugin** - Added to dependencies in `src-tauri/Cargo.toml`
2. **Update Server** - A server that hosts update manifests and files
3. **Signed Updates** (Recommended for production) - Code signing for security

## Configuration

### 1. Update Server Setup

The updater is configured to check for updates at:
```
https://api.attendease.app/updates/{{target}}/{{current_version}}
```

Where:
- `{{target}}` - Platform-specific identifier (e.g., `windows-x86_64`, `darwin-universal`)
- `{{current_version}}` - Current installed version

The endpoint should return a JSON response with update information:

```json
{
  "version": "1.1.0",
  "date": "2024-02-15",
  "body": "New features and bug fixes:\n- Added auto-update functionality\n- Improved UI performance",
  "signature": "(optional) signature for verification",
  "url": "https://api.attendease.app/downloads/attendease-1.1.0-windows-x86_64.msi"
}
```

### 2. Signing Updates (Production)

For production, updates should be signed:

1. **Generate a key pair**:
   ```bash
   cd src-tauri
   cargo tauri signer sign -w ~/.tauri/myapp.key
   ```

2. **Update `tauri.conf.json`** with your public key:
   ```json
   {
     "plugins": {
       "updater": {
         "pubkey": "YOUR_PUBLIC_KEY_HERE"
       }
     }
   }
   ```

3. **Sign your updates**:
   ```bash
   cargo tauri signer sign --private-key ~/.tauri/myapp.key path/to/update.msi
   ```

### 3. Build with Update Artifacts

When building for release, ensure update artifacts are created:

```bash
# Build with updater artifacts
cd src-tauri
cargo tauri build -- --config '{"bundle": {"createUpdaterArtifacts": true}}'

# This creates update manifests in src-tauri/target/release/bundle
```

## Update Workflow

### User Experience

1. **Auto-check on startup** - Application checks for updates every 4 hours
2. **Update notification** - Shows banner when updates are available
3. **Download progress** - Shows real-time progress during download
4. **Install and restart** - Automatic installation and application restart

### Settings Page

Users can configure update preferences in Settings:
- Toggle automatic update checking
- Manually check for updates
- View current and available versions
- Install updates manually

## Error Handling

The application includes robust error handling for:

- **Network issues** - Graceful fallback and retry mechanisms
- **Server errors** - Clear error messages and retry options
- **Corrupted downloads** - Automatic retry with validation
- **Permission issues** - User-friendly error messages

## Testing Updates

### Development Testing

For testing without a real update server:

1. **Mock update server** - Use local server or modify endpoints
2. **Test UI flow** - Verify notification and progress display
3. **Test error conditions** - Disconnect network to test error handling

### Update Server Testing

1. **Host update files** on a test server
2. **Configure test endpoint** in `tauri.conf.json`
3. **Increment version number** in `src-tauri/Cargo.toml`
4. **Build and test** the update flow

## Security Considerations

- **Always sign updates** in production
- **Use HTTPS** for update endpoints
- **Validate signatures** before installation
- **Monitor update servers** for availability and security
- **Rollback plan** for failed updates

## Troubleshooting

### Common Issues

1. **Update not showing**:
   - Check server endpoint configuration
   - Verify version numbers are incremented
   - Check network connectivity

2. **Download failures**:
   - Verify file URLs are accessible
   - Check permissions for installation directory
   - Ensure adequate disk space

3. **Installation failures**:
   - Check file permissions
   - Verify code signing certificates
   - Check antivirus software interference

### Debug Mode

Enable debug logging for troubleshooting:

```bash
# Enable debug logging
TAURI_DEBUG=1 cargo tauri dev

# Check updater status in dev tools
# Look for "update" related logs in console
```

## Maintenance

- **Regular updates** to the updater plugin
- **Monitor update server** performance and availability
- **Test update flow** after major changes
- **Keep signing certificates** secure and up-to-date
- **Update server endpoints** if needed

## Support

For issues with the auto-update functionality:
1. Check the error messages in the application
2. Verify network connectivity
3. Ensure sufficient disk space
4. Check application permissions
5. Review server logs for update requests