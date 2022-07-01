#!/bin/sh
set -eu

CURR_DIR="$(pwd)"
TMP_DIR="$(mktemp -d)"
cd "$TMP_DIR"

SSH_ARGS="-q -i ~/.ssh/conesnake_ed25519"

NUM_CLOUD_NODES="$(echo $CLOUD_NODES | jq length)"
NUM_REMOTE_NODES="$(echo $REMOTE_NODES | jq length)"

wg-meshconf init

# First configure with remote endpoints set to private IPs for cloud nodes

i=0
while [ $i -lt $NUM_CLOUD_NODES ]
do
    NODE_HOST="$(echo "$CLOUD_NODES" | jq -r '.['$i']["host"]')"
    NODE_ADDRESS="$(echo "$CLOUD_NODES" | jq -r '.['$i']["internal_ip"]')"
    NODE_PRIV_IP="$(echo "$CLOUD_NODES" | jq -r '.['$i']["private_ip"]')"

    echo "Creating wg config for node $NODE_HOST"

    wg-meshconf addpeer \
        --address "$NODE_ADDRESS/16" \
        --allowedips "$NODE_ADDRESS/32" \
        --listenport 59203 \
        --endpoint "$NODE_PRIV_IP" \
        --persistentkeepalive 15 \
        "$NODE_HOST"

    i=$(($i + 1))
done

i=0
while [ $i -lt $NUM_REMOTE_NODES ]
do
    NODE_HOST="$(echo "$REMOTE_NODES" | jq -r '.['$i']["host"]')"
    NODE_ADDRESS="$(echo "$REMOTE_NODES" | jq -r '.['$i']["internal_ip"]')"
    NODE_PORT="$(echo "$REMOTE_NODES" | jq -r '.['$i']["port"]')"

    echo "Creating wg config for node $NODE_HOST"

    wg-meshconf addpeer \
        --address "$NODE_ADDRESS/16" \
        --allowedips "$NODE_ADDRESS/32" \
        --listenport "$NODE_PORT" \
        --endpoint "$REMOTE_IP" \
        --persistentkeepalive 15 \
        "$NODE_HOST"

    i=$(($i + 1))
done


# Generate cloud configs, install on cloud nodes
i=0
while [ $i -lt $NUM_CLOUD_NODES ]
do
    NODE_HOST="$(echo "$CLOUD_NODES" | jq -r '.['$i']["host"]')"
    NODE_PUBLIC_IP="$(echo "$CLOUD_NODES" | jq -r '.['$i']["public_ip"]')"

    echo "Adding wg interface for node $NODE_HOST"

    wg-meshconf genconfig -o . "$NODE_HOST"

    while ! ssh $SSH_ARGS -o StrictHostKeyChecking=no "ubuntu@$NODE_PUBLIC_IP" echo '$NODE_HOST'
    do
        echo "Waiting for server..."
        sleep 1
    done

    TEMP_DIR="$(ssh $SSH_ARGS ubuntu@$NODE_PUBLIC_IP mktemp -d)"
    scp $SSH_ARGS "./$NODE_HOST.conf" "ubuntu@$NODE_PUBLIC_IP":"$TEMP_DIR"
    ssh $SSH_ARGS "ubuntu@$NODE_PUBLIC_IP" 'sh -s' <<EOF
set -eu
sudo mkdir -p /etc/wireguard
sudo cp $TEMP_DIR/$NODE_HOST.conf /etc/wireguard/conesnake.conf
sudo chmod 600 /etc/wireguard/conesnake.conf
sudo systemctl enable --now wg-quick@conesnake.service
rm -r $TEMP_DIR
EOF
    i=$(($i + 1))
done

# Update endpoints for remote nodes
i=0
while [ $i -lt $NUM_CLOUD_NODES ]
do
    NODE_HOST="$(echo "$CLOUD_NODES" | jq -r '.['$i']["host"]')"
    NODE_PUBLIC_IP="$(echo "$CLOUD_NODES" | jq -r '.['$i']["public_ip"]')"

    echo "Patching config for node $NODE_HOST"

    wg-meshconf updatepeer \
        --endpoint "$NODE_PUBLIC_IP" \
        "$NODE_HOST"

    i=$(($i + 1))
done

i=0
while [ $i -lt $NUM_REMOTE_NODES ]
do
    NODE_HOST="$(echo "$REMOTE_NODES" | jq -r '.['$i']["host"]')"
    NODE_PORT="$(echo "$REMOTE_NODES" | jq -r '.['$i']["port"]')"
    NODE_PRIV_IP="$(echo "$REMOTE_NODES" | jq -r '.['$i']["private_ip"]')"

    echo "Patching config for node $NODE_HOST"

    wg-meshconf updatepeer \
        --endpoint "$NODE_PRIV_IP" \
        "$NODE_HOST"

    i=$(($i + 1))
done

# Generate remote configs, install on remote nodes
i=0
while [ $i -lt $NUM_REMOTE_NODES ]
do
    NODE_HOST="$(echo "$REMOTE_NODES" | jq -r '.['$i']["host"]')"

    echo "Adding wg interface for node $NODE_HOST"

    wg-meshconf genconfig -o . "$NODE_HOST"

    TEMP_DIR="$(ssh $NODE_HOST mktemp -d)"
    scp "./$NODE_HOST.conf" "$NODE_HOST":"$TEMP_DIR"
    ssh "$NODE_HOST" 'sh -s' <<EOF
set -eu
sudo mkdir -p /etc/wireguard
sudo cp $TEMP_DIR/$NODE_HOST.conf /etc/wireguard/conesnake.conf
sudo chmod 600 /etc/wireguard/conesnake.conf
sudo systemctl enable --now wg-quick@conesnake.service
rm -r $TEMP_DIR
EOF

    i=$(($i + 1))
done

cd "$CURR_DIR"
rm -rf "$TMP_DIR"

echo "{}"
