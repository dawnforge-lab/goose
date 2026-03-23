use anyhow::Result;
use std::path::Path;

/// Install auto-start service for the current platform.
/// Linux: systemd user service
/// macOS: launchd plist
pub fn install_autostart() -> Result<()> {
    if cfg!(target_os = "linux") {
        install_systemd()
    } else if cfg!(target_os = "macos") {
        install_launchd()
    } else {
        tracing::info!("Auto-start not supported on this platform");
        Ok(())
    }
}

/// Remove auto-start service.
pub fn remove_autostart() -> Result<()> {
    if cfg!(target_os = "linux") {
        remove_systemd()
    } else if cfg!(target_os = "macos") {
        remove_launchd()
    } else {
        Ok(())
    }
}

fn install_systemd() -> Result<()> {
    let service_dir = dirs::home_dir()
        .expect("No home directory")
        .join(".config/systemd/user");
    std::fs::create_dir_all(&service_dir)?;

    let spawnbot_bin = spawnbot_common::paths::spawnbot_home()
        .join("bin")
        .join("spawnbot");
    let bin_path = if spawnbot_bin.exists() {
        spawnbot_bin.to_string_lossy().to_string()
    } else {
        // Fall back to PATH lookup
        "spawnbot".to_string()
    };

    let service = format!(
        r#"[Unit]
Description=Spawnbot autonomous agent daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={bin_path} start
Restart=on-failure
RestartSec=10
Environment=PATH={home}/.spawnbot/bin:{home}/.cargo/bin:/usr/local/bin:/usr/bin:/bin

[Install]
WantedBy=default.target
"#,
        bin_path = bin_path,
        home = dirs::home_dir().unwrap().display(),
    );

    let service_path = service_dir.join("spawnbot.service");
    std::fs::write(&service_path, service)?;

    // Enable the service
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "enable", "spawnbot.service"])
        .output();

    println!("  Auto-start installed (systemd user service)");
    println!("  Start now: systemctl --user start spawnbot");
    Ok(())
}

fn remove_systemd() -> Result<()> {
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "stop", "spawnbot.service"])
        .output();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "disable", "spawnbot.service"])
        .output();

    let service_path = dirs::home_dir()
        .unwrap()
        .join(".config/systemd/user/spawnbot.service");
    if service_path.exists() {
        std::fs::remove_file(&service_path)?;
    }

    let _ = std::process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    Ok(())
}

fn install_launchd() -> Result<()> {
    let agents_dir = dirs::home_dir()
        .expect("No home directory")
        .join("Library/LaunchAgents");
    std::fs::create_dir_all(&agents_dir)?;

    let spawnbot_bin = spawnbot_common::paths::spawnbot_home()
        .join("bin")
        .join("spawnbot");
    let bin_path = if spawnbot_bin.exists() {
        spawnbot_bin.to_string_lossy().to_string()
    } else {
        "spawnbot".to_string()
    };

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.dawnforge.spawnbot</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>start</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>{}/daemon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{}/daemon.stderr.log</string>
</dict>
</plist>
"#,
        bin_path,
        spawnbot_common::paths::spawnbot_home().display(),
        spawnbot_common::paths::spawnbot_home().display(),
    );

    let plist_path = agents_dir.join("com.dawnforge.spawnbot.plist");
    std::fs::write(&plist_path, plist)?;

    let _ = std::process::Command::new("launchctl")
        .args(["load", &plist_path.to_string_lossy()])
        .output();

    println!("  Auto-start installed (launchd)");
    Ok(())
}

fn remove_launchd() -> Result<()> {
    let plist_path = dirs::home_dir()
        .unwrap()
        .join("Library/LaunchAgents/com.dawnforge.spawnbot.plist");

    if plist_path.exists() {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &plist_path.to_string_lossy()])
            .output();
        std::fs::remove_file(&plist_path)?;
    }

    Ok(())
}
