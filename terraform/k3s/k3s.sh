#!/bin/sh
set -eu


INSTALL_FILES="\
  ./scripts/k3s_install.sh \
"

SERVER_INSTALL_FILES="\
  $INSTALL_FILES \
  ./k8s/cloud-provider-aws.yaml \
"


CLOUD_SSH_ARGS="-q -o StrictHostKeyChecking=no -i ~/.ssh/conesnake_ed25519"

uninstall_k3s_server()
{
  echo "Uninstalling k3s server $HOST..."
  ssh -q "$HOST" 'sh -s' <<EOF
k3s-uninstall.sh 2>/dev/null || true
EOF
}


uninstall_k3s_relay()
{
  echo "Uninstalling k3s relay $HOST..."
  ssh $CLOUD_SSH_ARGS "ubuntu@$PUBLIC_IP" 'sh -s' <<EOF
k3s-agent-uninstall.sh 2>/dev/null || true
EOF
}


uninstall_k3s_worker()
{
  echo "Uninstalling k3s worker $HOST..."
  ssh -q "$HOST" 'sh -s' <<EOF
k3s-agent-uninstall.sh 2>/dev/null || true
EOF
}

# ----------------

install_k3s_primary_server()
{
  echo "Installing k3s primary server..."
  TEMP_DIR="$(ssh -q $HOST mktemp -d)"

  scp -q $SERVER_INSTALL_FILES "$HOST":"$TEMP_DIR"
  ssh -q $HOST 'sh -s' <<EOF
set -eu
sudo mkdir -p /var/lib/rancher/k3s/server/manifests
sudo cp $TEMP_DIR/*.yaml /var/lib/rancher/k3s/server/manifests/
chmod +x $TEMP_DIR/k3s_install.sh
$TEMP_DIR/k3s_install.sh server \\
  --cluster-init \\
  --node-taint CriticalAddonsOnly=true:NoExecute \\
  --node-label mode=server \\
  --node-name $HOST \\
  --node-ip $INTERNAL_IP \\
  --flannel-iface conesnake \\
  --disable-cloud-controller \\
  --disable servicelb \\
  --disable traefik \\
  --kube-proxy-arg="--proxy-mode=ipvs" \\
  --kube-proxy-arg="--ipvs-scheduler=lc"

rm -r $TEMP_DIR
EOF
}


install_k3s_relay()
{
  echo "Installing k3s agent for relay node..."
  while ! ssh $CLOUD_SSH_ARGS "ubuntu@$PUBLIC_IP" echo '$HOST'
  do
    echo "Waiting for server..."
    sleep 1
  done


  TEMP_DIR="$(ssh $CLOUD_SSH_ARGS ubuntu@$PUBLIC_IP mktemp -d)"

  scp $CLOUD_SSH_ARGS $INSTALL_FILES "ubuntu@$PUBLIC_IP":"$TEMP_DIR"
  ssh $CLOUD_SSH_ARGS "ubuntu@$PUBLIC_IP" 'sh -s' <<EOF
set -eu
chmod +x $TEMP_DIR/k3s_install.sh
$TEMP_DIR/k3s_install.sh agent \\
  --node-label mode=relay \\
  --node-name $HOST \\
  --node-ip $INTERNAL_IP \\
  --server "https://$PRIMARY_INTERNAL_IP:6443" \\
  --token $K3S_TOKEN \\
  --flannel-iface conesnake \\
  --kube-proxy-arg="--proxy-mode=ipvs" \\
  --kube-proxy-arg="--ipvs-scheduler=lc" \\
  --kubelet-arg="provider-id=aws:///\\
\$(curl -s http://169.254.169.254/latest/meta-data/placement/availability-zone)/\\
\$(curl -s http://169.254.169.254/latest/meta-data/instance-id)"

rm -r $TEMP_DIR
EOF
}


install_k3s_worker()
{
  echo "Installing k3s agent for worker node..."
  TEMP_DIR="$(ssh -q $HOST mktemp -d)"
  scp -q $INSTALL_FILES "$HOST":"$TEMP_DIR"
  ssh -q "$HOST" 'sh -s' <<EOF
set -eu
chmod +x $TEMP_DIR/k3s_install.sh
$TEMP_DIR/k3s_install.sh agent \\
  --node-label mode=worker \\
  --node-name $HOST \\
  --node-ip $INTERNAL_IP \\
  --server "https://$PRIMARY_INTERNAL_IP:6443" \\
  --token $K3S_TOKEN \\
  --flannel-iface conesnake \\
  --kube-proxy-arg="--proxy-mode=ipvs" \\
  --kube-proxy-arg="--ipvs-scheduler=lc"

rm -r $TEMP_DIR
EOF
}

# ----------------

get_token()
{
  K3S_TOKEN="$( \
  ssh -q -t "$PRIMARY_HOST" \
    sudo cat /var/lib/rancher/k3s/server/node-token | sed 's/\r$//' \
  )"
}


install_config()
{
  KUBE_CONFIG="$(ssh -q $HOST sudo cat /etc/rancher/k3s/k3s.yaml | sed 's/127.0.0.1/'$INTERNAL_IP'/')"
  echo "$KUBE_CONFIG" > ~/.kube/config
  chmod 600 ~/.kube/config
}


# ----------------

OP="$1"
MODE="$2"

case "${OP} ${MODE}" in
  "destroy primary"|"destroy replica")
    uninstall_k3s_server
    ;;
  "destroy worker")
    uninstall_k3s_worker
    ;;
  "destroy relay")
    uninstall_k3s_relay
    ;;
  "create primary")
    install_k3s_primary_server
    install_config
    ;;
  "create worker")
    get_token
    install_k3s_worker
    ;;
  "create relay")
    get_token
    install_k3s_relay
    ;;
  *)
    echo "Unkown mode: $MODE"
    exit 1
    ;;
esac
