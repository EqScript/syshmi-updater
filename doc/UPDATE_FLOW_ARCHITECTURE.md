# SysHMI Kiosk Updater Architecture Research

## Overview
We are designing a Linux-based HMI kiosk with two main modules:
- **syshmi-core**: Backend (serial parsing) + frontend (eGUI).
- **syshmi-updater**: Handles update manifests, downloads, verification, and activation.

Goal: Clean CI/CD pipeline with safe remote updates, rollback, and optional remote maintenance.

---

## Current State
- **syshmi-core** runs as a `systemd` service under user `syshmi`.
- **syshmi-updater** fetches remote manifest, compares with local `current_version.toml`, and installs updates.
- Artifacts and manifests are stored in `/srv/firmware` with staging, releases, archive, and logs.

---

## Update Flow (User-Initiated)
1. **Manifest check**  
   - Updater polls remote manifest (signed, SHA256 verified).
   - Writes local state: `update_available`.

2. **Notification**  
   - Updater signals GUI (via D‑Bus or Unix Domain Socket).
   - GUI shows “Update available”.

3. **User consent**  
   - User clicks “Install update”.
   - GUI calls updater API (`StartUpdate`).

4. **Download & verify**  
   - Artifact fetched to `staging_dir`.
   - Verify checksum + detached signature.

5. **Activation (privileged helper)**  
   - Stop `syshmi-core`.
   - Atomic move/rename into `/srv/firmware/current`.
   - Update `current_version.toml`.
   - Restart `syshmi-core`.

6. **Rollback**  
   - Keep N previous versions in `archive`.
   - Provide `rollback` command to restore last good version.

---

## IPC Between GUI and Updater
- **Preferred:** D‑Bus interface (methods + signals).
  - Methods: `CheckUpdate()`, `StartUpdate(version)`, `Rollback()`.
  - Signals: `UpdateAvailable(version)`, `UpdateProgress(percent, stage)`, `UpdateFinished(success, message)`.
- **Alternative:** Unix Domain Socket with simple HTTP/JSON API.

---

## Privilege Model
- **Updater daemon:** Runs as `syshmi` user. Handles manifest polling, downloads, verification.
- **Privileged helper:** Minimal root oneshot service for:
  - Moving binaries into root-owned paths.
  - Writing WireGuard keys/secrets.
  - Starting/stopping system services.
- **Polkit rules:** Restrict privileged calls to updater/GUI only.

---

## Remote Maintenance
- **Default:** WireGuard VPN disabled.
- **Enable flow:**
  - User toggles “Allow remote maintenance” in GUI OR asserts GPIO pin.
  - Updater requests privileged helper to bring up WireGuard interface.
  - Ephemeral keys generated and applied.
  - Session auto-expires (15–60 min).
- **Audit:** Log enable/disable events with session ID, timestamps, and reason.

---

## Security Controls
- Signed manifests + SHA256 checksums.
- Ephemeral WireGuard keys for maintenance sessions.
- Atomic activation (rename + fsync).
- Rollback archives.
- Journal + local log files for updater and maintenance events.

---

## CI/CD Workflow
1. CI builds artifacts, computes SHA256, signs with GPG.
2. CI updates `manifest.toml` with version, URL, checksum, signature.
3. CI publishes artifacts + manifest to release repo.
4. Devices poll manifest, verify, and install updates.
5. Fleet dashboard collects telemetry (update success/failure, maintenance sessions).

---

## Next Steps
- Draft **systemd units** for updater (timer + service) and privileged helper.
- Define **D‑Bus interface spec** for GUI ↔ updater.
- Implement **ephemeral WireGuard key management**.
- Document **rollback procedure**.

