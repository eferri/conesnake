#!/bin/sh
set -eu

SSH_ARGS="-q -i ~/.ssh/conesnake_ed25519"

NUM_CLOUD_NODES="$(echo $CLOUD_NODES | jq length)"
NUM_REMOTE_NODES="$(echo $REMOTE_NODES | jq length)"

i=0
while [ $i -lt $NUM_CLOUD_NODES ]
do
    NODE_PUBLIC_IP="$(echo "$CLOUD_NODES" | jq -r '.['$i']["public_ip"]')"

    ssh $SSH_ARGS "ubuntu@$NODE_PUBLIC_IP" 'sh -s' <<EOF
set -eu
sudo systemctl disable --now wg-quick@conesnake.service
sudo wg-quick down conesnake || true
sudo rm -f /etc/wireguard/conesnake.conf
EOF

    i=$(($i + 1))
done


i=0
while [ $i -lt $NUM_REMOTE_NODES ]
do
    NODE_HOST="$(echo "$REMOTE_NODES" | jq -r '.['$i']["host"]')"

    ssh "$NODE_HOST" 'sh -s' <<EOF
set -eu
sudo systemctl disable --now wg-quick@conesnake.service
sudo wg-quick down conesnake || true
sudo rm -f /etc/wireguard/conesnake.conf
EOF

    i=$(($i + 1))
done
