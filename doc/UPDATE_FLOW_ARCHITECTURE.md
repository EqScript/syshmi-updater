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
   - Updater emits a D-Bus signal to the GUI.
   - GUI shows “Update available”.

3. **User consent**  
   - User clicks “Install update”.
   - GUI calls updater D-Bus method `StartUpdate(version)`.

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
   - Provide D-Bus method `Rollback()` to restore last good version.

---

## IPC Between GUI and Updater
Communication is implemented via **D-Bus (system bus)**.

### Methods
- `CheckUpdate()` → returns manifest info and whether update is available.
- `StartUpdate(version)` → initiates download, verification, and activation.
- `Rollback()` → restores previous version.

### Signals
- `UpdateAvailable(version)` → emitted when a new version is detected.
- `UpdateProgress(percent, stage)` → emitted during download/activation.
- `UpdateFinished(success, message)` → emitted when update completes.

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
- Define **D-Bus interface spec** (methods, signals, error codes).
- Implement **ephemeral WireGuard key management**.
- Document **rollback procedure**.


## D‑Bus Message Format

The `syshmi-core` UI communicates with `syshmi-updater` and system services via D‑Bus.  
Each button (Power Off, Reboot, Update) maps to a specific D‑Bus method call.

### Bus and Interface
- **Bus:** System bus
- **Service name:** `org.syshmi.Updater`
- **Object path:** `/org/syshmi/Updater`
- **Interface:** `org.syshmi.Updater`

### Methods
1. **CheckUpdate()**  
   Returns: `{ update_available: bool, version: string, message: string }`

2. **StartUpdate(string version)**  
   Initiates download, verification, and activation.  
   Returns: `{ success: bool, message: string }`  
   Emits signals during execution:  
   - `UpdateProgress(percent: int, stage: string)`  
   - `UpdateFinished(success: bool, message: string)`

3. **Rollback()**  
   Restores previous version.  
   Returns: `{ success: bool, message: string }`

4. **PowerOff()**  
   Requests systemd to power off the system.  
   Returns: `{ accepted: bool, message: string }`

5. **Reboot()**  
   Requests systemd to reboot the system.  
   Returns: `{ accepted: bool, message: string }`

### Signals
- `UpdateAvailable(version: string)` → emitted when a new version is detected.  
- `UpdateProgress(percent: int, stage: string)` → emitted during download/activation.  
- `UpdateFinished(success: bool, message: string)` → emitted when update completes.

### Error Codes
- `ERR_NO_UPDATE` → No update available.  
- `ERR_DOWNLOAD_FAILED` → Artifact could not be fetched.  
- `ERR_VERIFY_FAILED` → Checksum/signature mismatch.  
- `ERR_ACTIVATION_FAILED` → Failed to stop/start core or swap binaries.  
- `ERR_PRIVILEGE_REQUIRED` → Caller not authorized for privileged action.

---

### Example Call Flow (Update Button)
1. UI button “Update” → `StartUpdate("0.0.1b")`  
2. Updater downloads artifact, verifies checksum.  
3. Emits `UpdateProgress(30, "Downloading")`, `UpdateProgress(70, "Installing")`.  
4. Emits `UpdateFinished(true, "Update installed successfully")`.  
5. UI displays result.

