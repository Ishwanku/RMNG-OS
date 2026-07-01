#!/usr/bin/env bash
# Store LLM API key in ~/.rmng/secrets.env (chmod 600). Never commit.
# Usage:
#   ./scripts/setup-llm-secret.sh ANTHROPIC_API_KEY    # prompts securely
#   echo "$KEY" | ./scripts/setup-llm-secret.sh ANTHROPIC_API_KEY
set -euo pipefail

VAR_NAME="${1:-}"
if [[ -z "${VAR_NAME}" ]] || [[ ! "${VAR_NAME}" =~ ^[A-Z][A-Z0-9_]*$ ]]; then
  echo "Usage: setup-llm-secret.sh ENV_VAR_NAME"
  exit 1
fi

SECRETS="${HOME}/.rmng/secrets.env"
mkdir -p "${HOME}/.rmng"
touch "${SECRETS}"
chmod 600 "${SECRETS}"

if [[ -t 0 ]]; then
  read -rsp "Paste ${VAR_NAME} (hidden): " VALUE
  echo
else
  read -r VALUE
fi

if [[ -z "${VALUE}" ]]; then
  echo "ERROR: empty value"
  exit 1
fi

# Remove prior export of same var
grep -v "^export ${VAR_NAME}=" "${SECRETS}" > "${SECRETS}.tmp" 2>/dev/null || true
mv "${SECRETS}.tmp" "${SECRETS}"
echo "export ${VAR_NAME}=\"${VALUE}\"" >> "${SECRETS}"
chmod 600 "${SECRETS}"
echo "Wrote ${SECRETS} (${VAR_NAME})"
echo "Run: source ~/.bashrc && rmng provider health"