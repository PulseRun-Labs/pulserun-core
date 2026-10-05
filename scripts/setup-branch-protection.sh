#!/usr/bin/env bash
#
# Enable branch protection on `main` to match the standard for approved Stellar
# Wave repos: pull requests required, one approving review, and the CI jobs
# required to pass before merge.
#
# The required status-check contexts below MUST match the `name:` of the jobs in
# .github/workflows/ci.yml. If you rename a job there, update it here.
#
# Requires: gh CLI authenticated with admin rights on the repo.
# Usage:    ./scripts/setup-branch-protection.sh [owner/repo]
set -euo pipefail

REPO="${1:-PulseRun-Labs/pulserun-core}"

echo "==> Requiring pull requests + status checks on ${REPO}@main"
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "repos/${REPO}/branches/main/protection" \
  --input - <<'JSON'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["fmt · clippy · test", "build wasm32v1-none"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": true
}
JSON

echo "==> Done. Verify in Settings → Branches → main."
