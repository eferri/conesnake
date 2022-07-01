#!/bin/sh
set -eu

SSH_ARGS="-q -i ~/.ssh/conesnake_ed25519 -o ConnectTimeout=60"

uninstall_k3s_server()
{
  echo "Uninstalling k3s server $HOST..."
  ssh $SSH_ARGS "ubuntu@$PUBLIC_IP" 'sh -s' <<EOF
k3s-uninstall.sh || true
EOF
}


uninstall_k3s_relay()
{
  echo "Uninstalling k3s relay $HOST..."
  ssh $SSH_ARGS "ubuntu@$PUBLIC_IP" 'sh -s' <<EOF
k3s-agent-uninstall.sh || true
EOF
}


uninstall_k3s_worker()
{
  echo "Uninstalling k3s worker $HOST..."
  ssh -q "$HOST" 'sh -s' <<EOF
k3s-agent-uninstall.sh || true
EOF
}

case "${MODE}" in
  primary|replica)
    uninstall_k3s_server
    ;;
  worker)
    uninstall_k3s_worker
    ;;
  relay)
    uninstall_k3s_relay
    ;;
  *)
    echo "Unkown mode: $MODE"
    exit 1
    ;;
esac
