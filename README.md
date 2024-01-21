# conesnake
A tree search-based battlesnake.

## Developing
1. Prerequisites: docker and docker-compose

1. Build the docker containers and run a game locally:
    ```
    git submodule update --init --recursive
    cp example.env .env
    docker compose build
    docker compose up
    ```

1. View the locally running game. Look for link with the game ID printed by the rules service. For example:
    ```
    http://127.0.0.1:3000/?engine=http%3A%2F%2Flocalhost%3A4000&game=62befffb-711d-4ea1-99e6-3b6c17624d59
    ```

## Production

1. Create an GCP cloud bucket named "conesnake-tf-state" to store terraform state. Restrict public access

1. Generate ssh key for aws instance access:
    ```
    ssh-keygen -t ed25519 -C "" -f ~/.ssh/conesnake_ed25519 -N ""
    cat ~/.ssh/conesnake_ed25519.pub
    ```

1. Ensure all local nodes configured in terraform/vars.tf have password-less ssh access configured in ~/.ssh/config
   SSH host alias should match the `local_nodes`` map key in terraform/vars.tf

1. Install wireguard on all local nodes:
    ```
    ssh <local-node>
    sudo apt-get update && sudo apt-get install -no-install-recommends -y wireguard
    ```

1. Deploy infrastructure with terraform:
    ```
    cp terraform/vars.tf.example terraform/vars.tf
    make terraform-apply
    ```

1. Create a service account key for the "conesnake_registry" service account.
   Copy it to a file named `service_key.json` in the repo root

1. Create the regcred secret in k8s for pulling container images:
    ```
    make regcred-secret
    ```

1. Create a secrets file for the helm chart:
    ```
    cp k8s/conesnake/values.secrets.yaml.template k8s/conesnake/values.secrets.yaml
    ```

1. Build and push production docker images:
    ```
    make gcloud-config-docker
    make prod-build
    ```

1. Deploy the conesnake application with the helm chart:
    ```
    make helm-upgrade
    ```

## Profiling

1. Allow perf access to non-root user
    ```
    echo 'kernel.perf_event_paranoid=1' | sudo tee /etc/sysctl.d/perf.conf
    ```
1. Run the profile application with perf
    ```
    make profile
    ```
