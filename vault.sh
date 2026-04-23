#!/bin/bash
# vault.sh — Per-project encrypted APFS sparse bundle manager
#
# Encrypts any project directory using AES-256 APFS sparse bundles.
# Each project gets its own vault, stored in ~/.awl-vaults/.
#
# Usage:
#   vault.sh create <project-path>   Create new encrypted vault for a project
#   vault.sh open   <project-path>   Decrypt and mount vault at <project-path>
#   vault.sh close  <project-path>   Unmount and lock vault
#   vault.sh status [project-path]   Show vault state (default: cwd)
#   vault.sh compact <project-path>  Reclaim unused space (vault must be closed)
#   vault.sh backup <project-path> <dest>  Copy encrypted bundle to destination
#   vault.sh list                    List all known vaults
#
# Examples:
#   vault.sh create ~/myproject       # First-time setup for a project
#   vault.sh open ~/myproject         # Unlock before working
#   vault.sh close ~/myproject        # Lock when done
#   vault.sh status                  # Check vault state for cwd
#
# Security model:
#   - AES-256 encryption (APFS native, hardware-accelerated on Apple Silicon)
#   - Password prompted on each open — never stored in Keychain
#   - When closed, project directory is empty; data is fully inaccessible
#   - Encrypted bundles stored in ~/.awl-vaults/<project-name>.sparsebundle
#   - Each project has an independent password

set -euo pipefail

VAULTS_DIR="$HOME/.awl-vaults"
# 4GB max per vault — grows on demand as a sparse bundle
BUNDLE_SIZE="4g"

red()    { printf '\033[0;31m%s\033[0m\n' "$1"; }
green()  { printf '\033[0;32m%s\033[0m\n' "$1"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$1"; }

check_macos() {
    if [[ "$(uname)" != "Darwin" ]]; then
        red "Error: vault.sh requires macOS (hdiutil)."
        exit 1
    fi
}

# Resolve a project path to an absolute path and derive a vault name.
resolve_project() {
    local input="${1:-}"
    if [[ -z "$input" ]]; then
        red "Error: project path required."
        echo "  Usage: vault.sh <command> <project-path>"
        exit 1
    fi

    # Expand ~ and resolve to absolute path
    local abs_path
    local parent_dir
    parent_dir=$(dirname "$input")
    abs_path=$(cd "$parent_dir" 2>/dev/null && echo "$(pwd)/$(basename "$input")") || {
        red "Error: parent directory does not exist: $parent_dir"
        exit 1
    }

    # Derive vault name from the directory name
    local project_name
    project_name=$(basename "$abs_path")

    MOUNT_POINT="$abs_path"
    BUNDLE_PATH="$VAULTS_DIR/${project_name}.sparsebundle"
    VOLUME_NAME="${project_name}-vault"
}

is_mounted() {
    local info
    info=$(hdiutil info 2>/dev/null)
    echo "$info" | grep -qF "$BUNDLE_PATH" || echo "$info" | grep -qF "$MOUNT_POINT"
}

cmd_list() {
    mkdir -p "$VAULTS_DIR"
    local found=0
    echo "Known vaults in $VAULTS_DIR:"
    echo ""
    for bundle in "$VAULTS_DIR"/*.sparsebundle; do
        [[ -d "$bundle" ]] || continue
        found=1
        local name
        name=$(basename "$bundle" .sparsebundle)
        local size
        size=$(du -sh "$bundle" 2>/dev/null | cut -f1)

        # Check if this vault is currently mounted
        if hdiutil info 2>/dev/null | grep -q "$bundle"; then
            green "  $name  ($size)  OPEN"
        else
            echo "  $name  ($size)  closed"
        fi
    done
    if [[ $found -eq 0 ]]; then
        echo "  (none)"
        echo ""
        echo "Create one with: vault.sh create <project-path>"
    fi
}

cmd_status() {
    if is_mounted; then
        green "Vault '$VOLUME_NAME' is OPEN (mounted at $MOUNT_POINT)"
        df -h "$MOUNT_POINT" 2>/dev/null | tail -1
    else
        yellow "Vault '$VOLUME_NAME' is CLOSED (locked)"
        if [[ -d "$BUNDLE_PATH" ]]; then
            local size
            size=$(du -sh "$BUNDLE_PATH" 2>/dev/null | cut -f1)
            echo "  Bundle: $BUNDLE_PATH"
            echo "  Size on disk: $size"
        else
            echo "  No vault found for this project."
            echo "  Create one with: vault.sh create $MOUNT_POINT"
        fi
    fi
}

cmd_create() {
    if [[ -d "$BUNDLE_PATH" ]]; then
        red "Error: Vault already exists at $BUNDLE_PATH"
        echo "  To recreate, remove it first: rm -rf $BUNDLE_PATH"
        exit 1
    fi

    mkdir -p "$VAULTS_DIR"

    echo "Creating AES-256 encrypted vault for: $MOUNT_POINT"
    echo "Bundle will be stored at: $BUNDLE_PATH"
    echo ""
    echo "You will be prompted to set a password."
    yellow "IMPORTANT: Do NOT forget this password. There is no recovery mechanism."
    echo ""

    # If the project directory already has files, warn about migration
    if [[ -d "$MOUNT_POINT" ]] && [[ -n "$(ls -A "$MOUNT_POINT" 2>/dev/null)" ]]; then
        yellow "Warning: $MOUNT_POINT already contains files."
        echo "  After vault creation, these files will be HIDDEN by the mount."
        echo "  You must move them into the vault after it opens."
        echo ""
        read -rp "Continue? [y/N] " confirm
        if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
            echo "Aborted."
            exit 0
        fi
    fi

    # Create encrypted sparse bundle — password prompted interactively
    hdiutil create \
        -size "$BUNDLE_SIZE" \
        -encryption AES-256 \
        -type SPARSEBUNDLE \
        -fs APFS \
        -volname "$VOLUME_NAME" \
        "$BUNDLE_PATH"

    echo ""

    # Create mount point if needed and mount
    mkdir -p "$MOUNT_POINT"
    hdiutil attach "$BUNDLE_PATH" -mountpoint "$MOUNT_POINT"

    green "Vault created and mounted at $MOUNT_POINT"
    echo ""
    echo "Next steps:"
    echo "  1. cd $MOUNT_POINT"
    echo "  2. Add or copy your project files here"
    echo "  3. When done: vault.sh close $MOUNT_POINT"

    # Restrict vaults directory permissions
    chmod 700 "$VAULTS_DIR"
}

cmd_open() {
    if is_mounted; then
        yellow "Vault is already open at $MOUNT_POINT"
        exit 0
    fi

    if [[ ! -d "$BUNDLE_PATH" ]]; then
        red "Error: No vault found at $BUNDLE_PATH"
        echo "  Run: vault.sh create $MOUNT_POINT"
        exit 1
    fi

    # Create mount point if it doesn't exist
    mkdir -p "$MOUNT_POINT"

    echo "Opening vault for $(basename "$MOUNT_POINT") (password required)..."
    hdiutil attach "$BUNDLE_PATH" -mountpoint "$MOUNT_POINT"

    green "Vault open at $MOUNT_POINT"
    echo "  cd $MOUNT_POINT"
}

cmd_close() {
    if ! is_mounted; then
        yellow "Vault for $(basename "$MOUNT_POINT") is already closed."
        exit 0
    fi

    echo "Closing vault for $(basename "$MOUNT_POINT")..."

    # Force-unmount if processes have files open
    if ! hdiutil detach "$MOUNT_POINT" 2>/dev/null; then
        yellow "Mount point busy. Attempting force detach..."
        hdiutil detach "$MOUNT_POINT" -force
    fi

    green "Vault closed and locked: $(basename "$MOUNT_POINT")"
}

cmd_compact() {
    if is_mounted; then
        red "Error: Close the vault first (vault.sh close $MOUNT_POINT) before compacting."
        exit 1
    fi

    if [[ ! -d "$BUNDLE_PATH" ]]; then
        red "Error: No vault found at $BUNDLE_PATH"
        exit 1
    fi

    local before after
    before=$(du -sh "$BUNDLE_PATH" 2>/dev/null | cut -f1)
    echo "Compacting vault for $(basename "$MOUNT_POINT") (current size: $before)..."

    hdiutil compact "$BUNDLE_PATH"

    after=$(du -sh "$BUNDLE_PATH" 2>/dev/null | cut -f1)
    green "Compacted: $before -> $after"
}

cmd_backup() {
    local dest="${1:-}"
    if [[ -z "$dest" ]]; then
        red "Usage: vault.sh backup <project-path> <destination-path>"
        exit 1
    fi

    if is_mounted; then
        yellow "Warning: Backing up while vault is open. Close first for consistency."
        read -rp "Continue anyway? [y/N] " confirm
        if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
            echo "Aborted."
            exit 0
        fi
    fi

    if [[ ! -d "$BUNDLE_PATH" ]]; then
        red "Error: No vault found at $BUNDLE_PATH"
        exit 1
    fi

    if [[ ! -d "$dest" ]]; then
        red "Error: Destination does not exist: $dest"
        exit 1
    fi

    echo "Copying encrypted bundle to $dest ..."
    cp -R "$BUNDLE_PATH" "$dest/" || { red "Backup failed."; exit 1; }

    green "Backup complete: $dest/$(basename "$BUNDLE_PATH")"
    echo "The backup is encrypted — same password required to open."
}

usage() {
    cat <<'EOF'
vault.sh — Per-project encrypted vault manager

Usage:
  vault.sh create <project-path>          Create new AES-256 encrypted vault
  vault.sh open   <project-path>          Decrypt and mount
  vault.sh close  <project-path>          Unmount and lock
  vault.sh status [project-path]          Show vault state (default: cwd)
  vault.sh compact <project-path>         Reclaim unused disk space (must be closed)
  vault.sh backup <project-path> <dest>   Copy encrypted bundle elsewhere
  vault.sh list                           List all known vaults

Examples:
  vault.sh create ~/myproject        Create vault for ~/myproject
  vault.sh open ~/myproject          Unlock and mount
  cd ~/myproject                     Enter the decrypted project
  awl agent --task "..."             Use awl inside the project
  vault.sh close ~/myproject         Lock when done

Security:
  - Each project gets its own vault with its own password
  - Vaults stored in ~/.awl-vaults/<name>.sparsebundle
  - AES-256 with APFS — hardware-accelerated on Apple Silicon
  - Password prompted on each open (never stored in Keychain)
  - When closed, project directory is empty — data fully inaccessible
EOF
}

# --- Main ---
check_macos

case "${1:-}" in
    create)
        resolve_project "${2:-}"
        cmd_create
        ;;
    open)
        resolve_project "${2:-}"
        cmd_open
        ;;
    close)
        resolve_project "${2:-}"
        cmd_close
        ;;
    status)
        if [[ -n "${2:-}" ]]; then
            resolve_project "$2"
        else
            resolve_project "$(pwd)"
        fi
        cmd_status
        ;;
    compact)
        resolve_project "${2:-}"
        cmd_compact
        ;;
    backup)
        resolve_project "${2:-}"
        cmd_backup "${3:-}"
        ;;
    list)
        cmd_list
        ;;
    -h|--help|help|"")
        usage
        ;;
    *)
        red "Unknown command: $1"
        usage
        exit 1
        ;;
esac
