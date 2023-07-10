#!/bin/bash
mkdir -p ~/membrane
cat > ~/membrane/.config <<EOF
{
  "access_token": "$MCTL_TOKEN",
  "refresh_token": "v1.MTBpZ_mMXZucCsjiAKWTs7lqJWDCIuClXuIWYFd_eKi6EFbUYoHZikeK9qaMgnNWMs2W3hFOXzzaxrTSJAhZmRs",
  "scope": "offline_access",
  "token_type": "Bearer",
  "expires_at": 1691593067
}
EOF
