#!/bin/bash
# publish-prep.sh — Run this to finalize git state before pushing to GitHub.
# Review each step before running. Delete this script after use.
set -euo pipefail

echo "=== Awl GitHub Publishing Prep ==="
echo ""

lock_file=".git/index.lock"
if [[ -e "$lock_file" ]]; then
    echo "Git index lock detected at $lock_file."
    echo "If another git command is still running, wait for it to finish."
    echo "If a prior git command crashed, remove the stale lock and rerun:"
    echo "  rm $lock_file"
    exit 1
fi

# 1. Remove stale files from git tracking (old project artifacts)
echo "Removing stale tracked files..."
git rm --cached .DS_Store 2>/dev/null || true
git rm --cached -r hooks/ offline/ sum_sq 2>/dev/null || true

# 2. Stage all current files
echo "Staging current state..."
git add -A

# 3. Show what will be committed
echo ""
echo "=== Files to be committed ==="
git status --short
echo ""

# 4. Prompt for commit
read -rp "Commit with message 'chore: sanitize and prepare for public release'? [y/N] " confirm
if [[ "$confirm" == "y" || "$confirm" == "Y" ]]; then
    git commit -m "chore: sanitize and prepare for public release

- Remove dead Symbol.file field (redundant with tuple key)
- Remove unused RefKind::Call variant
- Add repository URL to Cargo.toml and README
- Generalize vault.sh examples
- Extend .gitignore
- Remove stale artifacts from prior project (hooks/, offline/, sum_sq)"
    echo ""
    echo "Committed. To push:"
    echo "  git remote add origin git@github.com:etwatson/awl.git"
    echo "  git push -u origin main"
else
    echo "Aborted. Changes are staged — run 'git commit' when ready."
fi

echo ""
echo "Optional: to squash the old sorting-benchmark history into a single root commit:"
echo "  git rebase -i --root"
echo "  (mark all commits except the first as 'squash')"
