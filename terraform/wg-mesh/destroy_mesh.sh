#!/bin/sh
set -eu

SSH_ARGS="-q -i ~/.ssh/conesnake_ed25519"

CLOUD_HOSTS="$(echo "$CLOUD_NODES" | jq -r 'keys[]')"
LOCAL_HOSTS="$(echo "$LOCAL_NODES" | jq -r 'keys[]')"

while IFS= read -r NODE_HOST
do
    echo "Uninstalling wg config for node $NODE_HOST"

    NODE_PUBLIC_IP="$(echo "$CLOUD_NODES" | jq -r '.["'$NODE_HOST'"]["public_ip"]')"

    ssh $SSH_ARGS "ubuntu@$NODE_PUBLIC_IP" 'sh -s' <<EOF
set -eu
sudo systemctl disable --now wg-quick@conesnake.service
sudo wg-quick down conesnake || true
sudo rm -f /etc/wireguard/conesnake.conf
EOF
done <<EOF
$CLOUD_HOSTS
EOF

while IFS= read -r NODE_HOST
do
    echo "Uninstalling wg config for node $NODE_HOST"

    ssh "$NODE_HOST" 'sh -s' <<EOF
set -eu
sudo systemctl disable --now wg-quick@conesnake.service
sudo wg-quick down conesnake || true
sudo rm -f /etc/wireguard/conesnake.conf
EOF
done <<EOF
$LOCAL_HOSTS
EOF
