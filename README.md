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

1. Create an S3 bucket named "conesnake-tf-state" to store terraform state. Restrict public access

1. Create a dynamodb table named "terraform_state" for state locking

1. Register or import domain into Route53

1. Generate a gpg key for encrypting iam keys. Use conesnake as the name:
    ```
    gpg --generate-key
    gpg --output iam-public-key.gpg --export conesnake
    ```

1. Generate ssh key for aws instance access:
    ```
    ssh-keygen -t ed25519 -C "" -f ~/.ssh/conesnake_ed25519 -N ""
    cat ~/.ssh/conesnake_ed25519.pub
    ```

1. Ensure all local nodes configured in terraform/vars.tf have password-less ssh access configured in ~/.ssh/config
   SSH host alias should match the `local_nodes`` map key in terraform/vars.tf

1. Install wireguard on all local nodes
    ```
    ssh <local-node>
    sudo apt-get update && sudo apt-get install -no-install-recommends -y wireguard
    ```

1. Deploy infrastructure with terraform:
    ```
    cp terraform/vars.tf.example terraform/vars.tf
    make terraform-apply
    ```

1. Create a secrets file for the helm chart.
   Use the `conesnake_access_key_id` value printed at the end of the command `make terraform-apply` in the previous step to fill in the value in `values.secrets.yaml`
   Use the `conesnake_target_group_arn` for the `aws_target_group_arn` value in `values.secrets.yaml`
   Decrypt the secret access key:

    ```
    cp k8s/conesnake/values.secrets.yaml.template k8s/conesnake/values.secrets.yaml
    make terraform-output-secret
    ```
   Use the decrypted secret access key for the `aws_secret_access_key` key in `values.secrets.yaml`

1. Build and push production docker images:
    ```
    make ecr-login
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
