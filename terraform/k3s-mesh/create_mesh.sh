#!/bin/sh
set -eu

./destroy_mesh.sh

CURR_DIR="$(pwd)"
SCRATCH_DIR="$(mktemp -d)"
cd "$SCRATCH_DIR"

SSH_ARGS="-q -i ~/.ssh/conesnake_ed25519"

CLOUD_HOSTS="$(echo "$CLOUD_NODES" | jq -r 'keys[]')"
LOCAL_HOSTS="$(echo "$LOCAL_NODES" | jq -r 'keys[]')"


wg-meshconf init

# First configure with remote endpoints set to private IPs for cloud nodes
while IFS= read -r NODE_HOST
do
    NODE_ADDRESS="$(echo "$CLOUD_NODES" | jq -r '.["'$NODE_HOST'"]["internal_ip"]')"
    NODE_PRIV_IP="$(echo "$CLOUD_NODES" | jq -r '.["'$NODE_HOST'"]["private_ip"]')"

    echo "Creating wg config for node $NODE_HOST"

    wg-meshconf addpeer \
        --address "$NODE_ADDRESS/32" \
        --listenport 59203 \
        --endpoint "$NODE_PRIV_IP" \
        --persistentkeepalive 15 \
        "$NODE_HOST"
done <<EOF
$CLOUD_HOSTS
EOF

while IFS= read -r NODE_HOST
do
    NODE_ADDRESS="$(echo "$LOCAL_NODES" | jq -r '.["'$NODE_HOST'"]["internal_ip"]')"
    NODE_PORT="$(echo "$LOCAL_NODES" | jq -r '.["'$NODE_HOST'"]["port"]')"

    echo "Creating wg config for node $NODE_HOST"

    wg-meshconf addpeer \
        --address "$NODE_ADDRESS/32" \
        --listenport "$NODE_PORT" \
        --endpoint "$LOCAL_IP" \
        --persistentkeepalive 15 \
        "$NODE_HOST"
done <<EOF
$LOCAL_HOSTS
EOF

# Generate cloud configs, install on cloud nodes
while IFS= read -r NODE_HOST
do
    NODE_PUBLIC_IP="$(echo "$CLOUD_NODES" | jq -r '.["'$NODE_HOST'"]["public_ip"]')"

    echo "Adding wg interface for node $NODE_HOST"

    wg-meshconf genconfig -o . "$NODE_HOST"

    while ! ssh $SSH_ARGS -o StrictHostKeyChecking=no "ubuntu@$NODE_PUBLIC_IP" echo 'server_alive'
    do
        echo "Waiting for server..."
        sleep 1
    done

    # Give ec2 instance time to start
    sleep 10

    TEMP_DIR="$(ssh $SSH_ARGS ubuntu@$NODE_PUBLIC_IP mktemp -d </dev/null)"
    scp $SSH_ARGS "./$NODE_HOST.conf" "ubuntu@$NODE_PUBLIC_IP":"$TEMP_DIR"
    ssh $SSH_ARGS "ubuntu@$NODE_PUBLIC_IP" 'sh -s' <<EOF
set -eu
sudo apt-get update
DEBIAN_FRONTEND='noninteractive' sudo apt-get install --no-install-recommends -y wireguard
sudo sed -i 's/#net.ipv4.ip_forward=1/net.ipv4.ip_forward=1/' /etc/sysctl.conf
sudo sysctl -p /etc/sysctl.conf
sudo mkdir -p /etc/wireguard
sudo cp $TEMP_DIR/$NODE_HOST.conf /etc/wireguard/conesnake.conf
sudo chmod 600 /etc/wireguard/conesnake.conf
sudo systemctl enable --now wg-quick@conesnake.service
rm -r $TEMP_DIR
EOF
done <<EOF
$CLOUD_HOSTS
EOF

# Update endpoints for local nodes
while IFS= read -r NODE_HOST
do
    NODE_PUBLIC_IP="$(echo "$CLOUD_NODES" | jq -r '.["'$NODE_HOST'"]["public_ip"]')"

    echo "Patching config for node $NODE_HOST"

    wg-meshconf updatepeer \
        --endpoint "$NODE_PUBLIC_IP" \
        "$NODE_HOST"
done <<EOF
$CLOUD_HOSTS
EOF

while IFS= read -r NODE_HOST
do
    NODE_PORT="$(echo "$LOCAL_NODES" | jq -r '.["'$NODE_HOST'"]["port"]')"
    NODE_PRIV_IP="$(echo "$LOCAL_NODES" | jq -r '.["'$NODE_HOST'"]["private_ip"]')"

    echo "Patching config for node $NODE_HOST"

    wg-meshconf updatepeer \
        --endpoint "$NODE_PRIV_IP" \
        "$NODE_HOST"
done <<EOF
$LOCAL_HOSTS
EOF

# Generate local configs, install on local nodes
while IFS= read -r NODE_HOST
do
    echo "Adding wg interface for node $NODE_HOST"

    wg-meshconf genconfig -o . "$NODE_HOST"

    TEMP_DIR="$(ssh "$NODE_HOST" mktemp -d </dev/null)"
    scp "./$NODE_HOST.conf" "$NODE_HOST":"$TEMP_DIR"
    ssh "$NODE_HOST" 'sh -s' <<EOF
set -eu
sudo mkdir -p /etc/wireguard
sudo cp $TEMP_DIR/$NODE_HOST.conf /etc/wireguard/conesnake.conf
sudo chmod 600 /etc/wireguard/conesnake.conf
sudo systemctl enable --now wg-quick@conesnake.service
rm -r $TEMP_DIR
EOF
done <<EOF
$LOCAL_HOSTS
EOF

cd "$CURR_DIR"
rm -rf "$SCRATCH_DIR"
