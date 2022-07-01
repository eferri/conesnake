# Docker
AWS_ACCOUNT_ID ?= $(shell aws --no-cli-pager sts get-caller-identity --query 'Account')
AWS_REGION ?= $(shell aws --no-cli-pager configure get region)

.PHONY: shell
shell:
	docker compose run --rm snake bash

.PHONY: root-shell
root-shell:
	docker compose run --user root --rm snake bash

.PHONY: ecr-login
ecr-login:
	aws ecr get-login-password | docker login \
		--username AWS \
		--password-stdin "$(AWS_ACCOUNT_ID)".dkr.ecr.$(AWS_REGION).amazonaws.com

.PHONY: prod-build
prod-build:
	docker compose run --rm snake cargo build --release
	DOCKER_BUILDKIT=1 docker build \
		--target prod \
		--tag $(AWS_ACCOUNT_ID).dkr.ecr.$(AWS_REGION).amazonaws.com/conesnake:latest-app .

	DOCKER_BUILDKIT=1 docker build \
		--target job \
		--tag $(AWS_ACCOUNT_ID).dkr.ecr.$(AWS_REGION).amazonaws.com/conesnake:latest-job .

	docker push $(AWS_ACCOUNT_ID).dkr.ecr.$(AWS_REGION).amazonaws.com/conesnake:latest-app
	docker push $(AWS_ACCOUNT_ID).dkr.ecr.$(AWS_REGION).amazonaws.com/conesnake:latest-job

# Misc

.PHONY: clean
clean:
	rm -f *.log *.pb *.svg .k8s/manifest.yaml

.PHONY: veryclean
veryclean: clean
	rm -rf target* build* \
		.cargo/registry \
		.cargo/package-cache \
		*.tfstate*

# Cargo

.PHONY: test
test:
	docker compose run --rm snake cargo test -- \
		--nocapture \
		--color always

.PHONY: rel-test
rel-test:
	docker compose run --rm snake cargo test \
		--release -- \
		--nocapture \
		--color always \
		--test-threads=1

.PHONY: profile
profile:
	docker compose run --rm snake cargo test \
		--profile=release-with-debug \
		--features profile \
		arcade_maze_profile_test -- \
		--nocapture \
		--color always

.PHONY: bench
bench:
	docker compose run --rm snake cargo test \
		--release \
		arcade_maze_profile_test -- \
		--nocapture \
		--color always

# k3s deployment

.PHONY: helm-upgrade
helm-upgrade:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/rust/.kube" snake \
	helm upgrade prod conesnake \
		--install \
		--create-namespace \
		--namespace conesnake \
		--values conesnake/values.secrets.yaml

.PHONY: helm-uninstall
helm-uninstall:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/rust/.kube" snake \
	helm uninstall prod --namespace conesnake

.PHONY: helm-template
helm-template:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/rust/.kube" snake \
	helm template \
		--values conesnake/values.secrets.yaml \
		--debug prod conesnake > k8s/manifest.yaml

.PHONY: helm-lint
helm-lint:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/rust/.kube" snake \
	helm lint conesnake \
		--values conesnake/values.secrets.yaml \

# terraform

.PHONY: terraform-apply
terraform-apply:
	docker compose run --rm -w /app/terraform \
		-v "$(HOME)/.aws:/home/rust/.aws" \
		-v "$(HOME)/.ssh:/home/rust/.ssh" \
		-v "$(HOME)/.kube:/home/rust/.kube" \
		snake \
	terraform apply

.PHONY: terraform-init
terraform-init:
	docker compose run --rm -w /app/terraform \
		-v "$(HOME)/.aws:/home/rust/.aws" \
		snake \
	terraform init -upgrade

.PHONY: terraform-destroy
terraform-destroy:
	docker compose run --rm -w /app/terraform \
		-v "$(HOME)/.aws:/home/rust/.aws" \
		-v "$(HOME)/.ssh:/home/rust/.ssh" \
		-v "$(HOME)/.kube:/home/rust/.kube" \
		snake \
	terraform apply -destroy
