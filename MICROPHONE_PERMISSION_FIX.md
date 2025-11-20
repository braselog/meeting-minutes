# Microphone Permission Fix for macOS

## Problem
The app was not requesting microphone permissions on macOS, causing microphone audio to not be captured during recording. While system audio (speakers) worked fine, microphone input was completely silent. The app worked when run from Terminal because Terminal had microphone permissions.

## Root Cause
The application had:
- ✅ Proper `NSMicrophoneUsageDescription` in `Info.plist`
- ✅ Proper entitlements in `entitlements.plist`
- ❌ **Missing**: Actual code to trigger the microphone permission dialog
- ❌ **Missing**: Permission check before starting recording

On macOS, having the Info.plist entry alone is not enough - you must **actively attempt to access the microphone** to trigger the system permission dialog.

## Solution Implemented

### 1. Added Microphone Permission Functions (`audio/permissions.rs`)

#### New Functions:
- `check_microphone_permission()` - Check if microphone access is granted
- `request_microphone_permission()` - Trigger the permission dialog by attempting to access microphone
- `ensure_microphone_permission()` - Check and request if needed (returns true if granted)
- `init_microphone_permission()` - Initialize permission request on app startup

#### New Tauri Commands (exposed to frontend):
- `check_microphone_permission_command`
- `request_microphone_permission_command`
- `ensure_microphone_permission_command`

### 2. Integration Points

#### App Startup (`lib.rs`)
```rust
// Initialize microphone permission on startup
// This will request permission early so it's ready when recording starts
audio::init_microphone_permission();
```

This triggers a background thread that:
1. Waits 500ms for app initialization
2. Checks if microphone permission is granted
3. If not, attempts to access microphone to trigger the system dialog

#### Recording Start (`audio/recording_commands.rs`)
Added permission checks to all recording functions:
- `start_recording_with_meeting_name()`
- `start_recording_with_devices_and_meeting()`

```rust
#[cfg(target_os = "macos")]
{
    if !crate::audio::ensure_microphone_permission() {
        return Err("Microphone permission is required...".to_string());
    }
}
```

### 3. Permission Flow

#### First App Launch:
1. App starts → `init_microphone_permission()` called
2. After 500ms → checks microphone permission
3. If not granted → attempts to access microphone
4. **macOS shows permission dialog automatically**
5. User grants permission
6. Recording works

#### Subsequent Launches:
1. App starts → `init_microphone_permission()` called
2. Permission already granted → no dialog shown
3. Recording works immediately

#### If User Denies Permission:
1. Recording start attempted
2. `ensure_microphone_permission()` returns `false`
3. Recording fails with clear error message:
   > "Microphone permission is required to record audio. Please grant permission in System Settings > Privacy & Security > Microphone and restart the app."

## Technical Details

### How Permission Triggering Works
The `request_microphone_permission()` function uses the existing `trigger_audio_permission()` from the devices module, which:

1. Gets the default input device using `cpal`
2. Attempts to build an audio input stream
3. Starts the stream briefly (100ms)
4. This triggers macOS to show the permission dialog if not already granted

```rust
pub fn trigger_audio_permission() -> Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device()?;
    let config = device.default_input_config()?;
    
    let stream = device.build_input_stream(
        &config.into(),
        |_data: &[f32], _: &cpal::InputCallbackInfo| {},
        |err| error!("Error in audio stream: {}", err),
        None,
    )?;
    
    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(stream);
    
    Ok(())
}
```

### Files Modified

1. **`frontend/src-tauri/src/audio/permissions.rs`**
   - Added microphone permission functions
   - Added Tauri commands for frontend integration
   - Added initialization function for startup

2. **`frontend/src-tauri/src/audio/mod.rs`**
   - Exported new permission functions and commands

3. **`frontend/src-tauri/src/audio/recording_commands.rs`**
   - Added permission checks at recording start
   - Provides clear error messages if permission denied

4. **`frontend/src-tauri/src/lib.rs`**
   - Called `init_microphone_permission()` during app setup
   - Added permission commands to invoke handler

### Existing Configuration (Already Present)

**`frontend/src-tauri/Info.plist`:**
```xml
<key>NSMicrophoneUsageDescription</key>
<string>This application needs access to your microphone to record meeting audio.</string>
```

**`frontend/src-tauri/entitlements.plist`:**
```xml
<key>com.apple.security.device.audio-input</key>
<true/>
<key>com.apple.security.device.microphone</key>
<true/>
```

## Testing Recommendations

### Test Case 1: Fresh Install
1. Install DMG on a Mac that has never run the app
2. Launch the app
3. **Expected**: Microphone permission dialog appears within 1 second
4. Grant permission
5. Start a recording
6. **Expected**: Microphone audio is captured successfully

### Test Case 2: Permission Denied
1. Launch app
2. Deny microphone permission when dialog appears
3. Try to start a recording
4. **Expected**: Clear error message about needing permission
5. Go to System Settings > Privacy & Security > Microphone
6. Enable permission for the app
7. Restart app and try recording
8. **Expected**: Recording works

### Test Case 3: Permission Already Granted
1. Launch app with microphone permission already granted
2. Start recording immediately
3. **Expected**: No permission dialog, recording works immediately

### Test Case 4: Manual Permission Request
Frontend can also manually trigger permission request:
```typescript
await invoke('request_microphone_permission_command');
```

## Additional Notes

### Why This Bug Existed
The app was checking for "Screen Recording" permission (needed for system audio capture via ScreenCaptureKit), but **not** checking for microphone permission. These are two separate permissions on macOS:

- **Screen Recording Permission** → Required for `ScreenCaptureKit` to capture system audio
- **Microphone Permission** → Required for `cpal`/Core Audio to access microphone input

### Platform Differences
- **macOS**: Requires explicit permission request (fixed by this PR)
- **Windows**: No permission dialog needed
- **Linux**: No permission dialog needed

The code includes `#[cfg(target_os = "macos")]` to only apply these checks on macOS.

## User Experience Improvements

1. **Proactive Permission Request**: Permission is requested on app startup, not when user tries to record
2. **Clear Error Messages**: If permission is denied, user gets a helpful message with instructions
3. **No Blocking**: Permission request happens in background thread, doesn't delay app startup
4. **Frontend Control**: Frontend can check and request permissions programmatically if needed

## Related Files

- Original Issue: The user reported mic not being recorded when app installed via DMG
- Test Environment: macOS Sequoia 15.7.2, App Version 0.1.1
- Key Insight: App worked when run from Terminal because Terminal had microphone permissions

## Build Instructions

After applying this fix, rebuild the app:

```bash
cd frontend
pnpm install
pnpm tauri build
```

The new DMG will include the microphone permission handling.
