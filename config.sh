#!/bin/bash
mkdir -p ~/membrane
cat > ~/membrane/.config <<EOF
{
  "access_token": "$MCTL_TOKEN",
  "refresh_token": "v1.Me6CPegBsO8dPyUElrVzqtiu6XWco008-jIOgnXZo3cJaYLuijn9E_PgA1lgzap47PP99I6McAsC8k7JzhV-Lpk",
  "scope": "offline_access",
  "token_type": "Bearer",
  "expires_at": 1691679583
}
EOF
