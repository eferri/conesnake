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

Production images are built by CirceCi. They can also be manually built and pushed with `make ecr-login && make prod-build`

1. Setup AWS account for terraform. Requires state bucket and dynamodb table for state locking

1. Register or import domain into Route53

1. Generate gpg key for encrypting iam keys:
    ```
    gpg --generate-key
    gpg --output iam-public-key.gpg --export conesnake
    ```

1. Generate ssh key for instance access:
    ```
    ssh-keygen -t ed25519 -C "" -f ~/.ssh/conesnake_ed25519 -N ""
    cat ~/.ssh/conesnake_ed25519.pub
    ```

1. Deploy infrastructure with terraform:
    ```
    cp terraform/vars.tf.example terraform/vars.tf
    make terraform-apply
    ```

1. Decrypt iam keys using gpg key:
    ```
    make terraform-output
    echo <encrypted_secret_access_key> | base64 --decode | gpg --decrypt
    ```

1. Deploy the helm chart:
    ```
    cp k8s/conesnake/values.secrets.yaml.template k8s/conesnake/values.secrets.yaml
    make helm-deploy
    ```
## Profiling

1. Allow perf access to non-root user
    ```
    echo 'kernel.perf_event_paranoid=1' | sudo tee /etc/sysctl.d/perf.conf
    ```
