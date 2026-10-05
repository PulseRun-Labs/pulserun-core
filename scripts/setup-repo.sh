#!/usr/bin/env bash
#
# Apply repo-level settings for a well-run public repo: discoverability topics
# and branch protection on `main`.
#
# NOTE ON PERMISSIONS: both operations require the `administration` permission.
# A fine-grained integration token can hold `admin: true` on the repo and still
# be refused here with "Resource not accessible by integration". If that happens,
# run this script with a personal access token (classic `repo`+`admin:repo_hook`,
# or fine-grained with Administration: write).
#
# The required status-check contexts MUST match the `name:` fields in
# .github/workflows/ci.yml. Rename a job there, update it here.
#
# Requires: gh CLI authenticated with admin rights.
# Usage:    ./scripts/setup-repo.sh [owner/repo]
set -euo pipefail

REPO="${1:-PulseRun-Labs/pulserun-core}"

echo "==> Setting repository topics on ${REPO}"
gh api --method PUT "repos/${REPO}/topics" \
  -f "names[]=stellar" \
  -f "names[]=soroban" \
  -f "names[]=blockchain" \
  -f "names[]=rust" \
  -f "names[]=smart-contracts" \
  -f "names[]=escrow" \
  -f "names[]=defi" \
  -f "names[]=ci-cd" \
  -f "names[]=web3" \
  -f "names[]=pay-per-use" \
  --jq '.names'

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

echo "==> Done. Verify topics on the repo page and protection in Settings → Branches."
