// macOS audio permissions handling
use anyhow::Result;
use log::{info, warn, error};

#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
use std::sync::Once;

#[cfg(target_os = "macos")]
static INIT_MICROPHONE_PERMISSION: Once = Once::new();

/// Check if the app has Audio Capture permission (required for Core Audio taps on macOS 14.4+)
///
/// Note: Core Audio taps require NSAudioCaptureUsageDescription in Info.plist.
/// When the app first attempts to create a Core Audio tap, macOS will automatically
/// show a permission dialog to the user. If permission is denied, the tap will return
/// silence (all zeros).
///
/// This function returns true because the actual permission prompt happens automatically
/// when AudioHardwareCreateProcessTap is called by the cidre library.
#[cfg(target_os = "macos")]
pub fn check_screen_recording_permission() -> bool {
    info!("ℹ️  Core Audio tap requires Audio Capture permission (macOS 14.4+)");
    info!("📍 Permission dialog will appear automatically when recording starts");
    info!("   If already granted: System Settings → Privacy & Security → Audio Capture");

    // Always return true - the actual permission dialog is triggered by Core Audio API
    true
}

#[cfg(not(target_os = "macos"))]
pub fn check_screen_recording_permission() -> bool {
    true // Not required on other platforms
}

/// Request Audio Capture permission from the user
/// This will open System Settings to the Privacy & Security page
#[cfg(target_os = "macos")]
pub fn request_screen_recording_permission() -> Result<()> {
    info!("🔐 Opening System Settings for Audio Capture permission...");

    // Open System Settings to Privacy & Security page
    // Note: There's no direct URL for Audio Capture, so we open the main Privacy page
    let result = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security")
        .spawn();

    match result {
        Ok(_) => {
            info!("✅ Opened System Settings - navigate to Privacy & Security → Audio Capture");
            info!("👉 Please enable Audio Capture permission and restart the app");
            Ok(())
        }
        Err(e) => {
            error!("❌ Failed to open System Settings: {}", e);
            Err(anyhow::anyhow!("Failed to open System Settings: {}", e))
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_screen_recording_permission() -> Result<()> {
    Ok(()) // Not required on other platforms
}

/// Check and request Audio Capture permission if not granted
/// Returns true if permission is granted, false otherwise
pub fn ensure_screen_recording_permission() -> bool {
    if check_screen_recording_permission() {
        return true;
    }

    warn!("Audio Capture permission not granted - requesting...");

    if let Err(e) = request_screen_recording_permission() {
        error!("Failed to request Audio Capture permission: {}", e);
        return false;
    }

    false // Permission will be granted after restart
}

/// Tauri command to check Screen Recording permission
#[tauri::command]
pub async fn check_screen_recording_permission_command() -> bool {
    check_screen_recording_permission()
}

/// Tauri command to request Screen Recording permission
#[tauri::command]
pub async fn request_screen_recording_permission_command() -> Result<(), String> {
    request_screen_recording_permission()
        .map_err(|e| e.to_string())
}

/// Trigger system audio permission request programmatically
/// This attempts to create a Core Audio tap to trigger the Audio Capture permission dialog
#[cfg(target_os = "macos")]
pub fn trigger_system_audio_permission() -> Result<()> {
    info!("🔐 Triggering Audio Capture permission request...");

    // Try to create a Core Audio capture - this automatically triggers the permission dialog
    // if NSAudioCaptureUsageDescription is present in Info.plist
    match crate::audio::capture::CoreAudioCapture::new() {
        Ok(capture) => {
            info!("✅ Core Audio capture created, attempting to create stream...");

            // Try to create a stream - this is what actually triggers the permission dialog
            match capture.stream() {
                Ok(_stream) => {
                    info!("✅ Audio Capture permission already granted - stream created successfully");
                    Ok(())
                }
                Err(e) => {
                    // Check if this is a permission error
                    let error_msg = e.to_string().to_lowercase();
                    if error_msg.contains("permission") || error_msg.contains("audio") {
                        info!("🔐 Audio Capture permission dialog should have appeared");
                        info!("👉 Please grant Audio Capture permission and restart the app");
                        Ok(()) // This is expected - we triggered the dialog
                    } else {
                        warn!("⚠️ Failed to create system audio stream: {}", e);
                        Err(e)
                    }
                }
            }
        }
        Err(e) => {
            // Check if this is a permission error
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("permission") || error_msg.contains("audio") {
                info!("🔐 Audio Capture permission dialog should have appeared");
                info!("👉 Please grant Audio Capture permission and restart the app");
                Ok(()) // This is expected - we triggered the dialog
            } else {
                warn!("⚠️ Failed to trigger Audio Capture permission: {}", e);
                Err(e)
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_system_audio_permission() -> Result<()> {
    // System audio permissions not required on other platforms
    info!("System audio permissions not required on this platform");
    Ok(())
}

/// Tauri command to trigger system audio permission request
#[tauri::command]
pub async fn trigger_system_audio_permission_command() -> Result<(), String> {
    trigger_system_audio_permission()
        .map_err(|e| e.to_string())
}

/// Check if the app has microphone permission
/// This uses cpal to attempt to enumerate input devices, which triggers the permission dialog
#[cfg(target_os = "macos")]
pub fn check_microphone_permission() -> bool {
    use cpal::traits::HostTrait;
    
    info!("🎤 Checking microphone permission...");
    
    let host = cpal::default_host();
    
    // Try to get the default input device
    match host.default_input_device() {
        Some(device) => {
            info!("✅ Microphone permission granted - default input device available");
            true
        }
        None => {
            warn!("⚠️ No default microphone device available - permission may not be granted");
            false
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_microphone_permission() -> bool {
    true // Not required on other platforms
}

/// Request microphone permission from the user
/// This triggers the permission dialog by attempting to access the microphone
#[cfg(target_os = "macos")]
pub fn request_microphone_permission() -> Result<()> {
    info!("🔐 Requesting microphone permission...");
    
    // Use the existing trigger_audio_permission function from devices module
    // This will attempt to create an audio stream which triggers the permission dialog
    match crate::audio::trigger_audio_permission() {
        Ok(_) => {
            info!("✅ Microphone permission request triggered successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Failed to trigger microphone permission: {}", e);
            Err(e)
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone_permission() -> Result<()> {
    Ok(()) // Not required on other platforms
}

/// Ensure microphone permission is granted
/// This will request permission if not already granted
/// Returns true if permission is already granted, false if it was just requested
#[cfg(target_os = "macos")]
pub fn ensure_microphone_permission() -> bool {
    if check_microphone_permission() {
        info!("✅ Microphone permission already granted");
        return true;
    }
    
    info!("⚠️ Microphone permission not granted - requesting...");
    
    if let Err(e) = request_microphone_permission() {
        error!("❌ Failed to request microphone permission: {}", e);
        return false;
    }
    
    // Check again after requesting
    let granted = check_microphone_permission();
    if granted {
        info!("✅ Microphone permission granted after request");
    } else {
        warn!("⚠️ Microphone permission still not granted after request");
    }
    
    granted
}

#[cfg(not(target_os = "macos"))]
pub fn ensure_microphone_permission() -> bool {
    true // Not required on other platforms
}

/// Tauri command to check microphone permission
#[tauri::command]
pub async fn check_microphone_permission_command() -> bool {
    check_microphone_permission()
}

/// Tauri command to request microphone permission
#[tauri::command]
pub async fn request_microphone_permission_command() -> Result<(), String> {
    request_microphone_permission()
        .map_err(|e| e.to_string())
}

/// Tauri command to ensure microphone permission (check and request if needed)
#[tauri::command]
pub async fn ensure_microphone_permission_command() -> bool {
    ensure_microphone_permission()
}

/// Initialize and request microphone permission on app startup
/// This should be called during app setup to ensure permissions are requested early
#[cfg(target_os = "macos")]
pub fn init_microphone_permission() {
    INIT_MICROPHONE_PERMISSION.call_once(|| {
        info!("🎤 Initializing microphone permission on app startup...");
        
        // Spawn a thread to avoid blocking startup
        std::thread::spawn(|| {
            // Small delay to ensure app is fully initialized
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            if !check_microphone_permission() {
                info!("🔐 Microphone permission not granted, requesting...");
                let _ = request_microphone_permission();
            } else {
                info!("✅ Microphone permission already granted");
            }
        });
    });
}

#[cfg(not(target_os = "macos"))]
pub fn init_microphone_permission() {
    // Not required on other platforms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_permission() {
        let has_permission = check_screen_recording_permission();
        println!("Has Screen Recording permission: {}", has_permission);
    }
    
    #[test]
    fn test_check_microphone_permission() {
        let has_permission = check_microphone_permission();
        println!("Has Microphone permission: {}", has_permission);
    }
}