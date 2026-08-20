#!/usr/bin/env bash
set -euo pipefail
systemctl_user() {
  if systemctl --user show-environment >/dev/null 2>&1; then
    systemctl --user "$@"
  else
    systemctl --user --machine="${USER}@.host" "$@"
  fi
}
systemctl_user start arda-mirromere.service
systemctl_user is-active --quiet arda-mirromere.service
printf 'Mirromere started explicitly\n'
