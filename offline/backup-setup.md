# Backup Configuration

## Pre-Agent Hooks (automatic)

When `claw agent --offline` runs, two hooks fire before the agent loop:

1. **`hooks/snapshot.sh`** — Creates an APFS local snapshot via `tmutil localsnapshot`.
2. **`hooks/pre-agent.sh`** — Commits all uncommitted changes in the working directory as a pre-agent snapshot.

Both hooks are best-effort and will not block the agent if they fail.

## Restic to External Drive

```bash
# Initialize (one time)
restic -r /Volumes/Backup/claw-restic init

# Daily backup (add to crontab or launchd)
restic -r /Volumes/Backup/claw-restic backup ~/claw ~/.config/claw ~/.gpd
restic -r /Volumes/Backup/claw-restic forget --keep-daily 7 --keep-weekly 4
```

## Recovery Test

```bash
# List snapshots
restic -r /Volumes/Backup/claw-restic snapshots

# Restore latest to /tmp/recovery-test
restic -r /Volumes/Backup/claw-restic restore latest --target /tmp/recovery-test

# Verify
diff -r ~/claw /tmp/recovery-test/claw
```

## Time Machine

Time Machine is configured for daily backups to an external drive. Verify with:

```bash
tmutil listlocalsnapshots / | tail -3
```

## Recovery Priority

1. **Git** — fastest, covers all committed code: `git checkout -- <file>`
2. **APFS snapshot** — covers pre-agent state: `tmutil restore`
3. **Time Machine** — covers daily state
4. **Restic** — covers `~/claw`, `~/.config/claw`, `~/.gpd`
