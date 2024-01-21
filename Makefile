# Docker
GCP_REGISTRY_SA_EMAIL := $(shell gcloud iam service-accounts list --filter=displayName="conesnake_registry_service_account" --format="get(email)" 2>/dev/null)
GCP_PROJECT_NAME := $(shell gcloud config get-value project)

.PHONY: shell
shell:
	docker compose run --rm \
		-v "$(HOME)/.config/gcloud:/home/conesnake/.config/gcloud" \
		-v "$(HOME)/.ssh:/home/conesnake/.ssh" \
		-v "$(HOME)/.kube:/home/conesnake/.kube" \
	snake bash

.PHONY: root-shell
root-shell:
	docker compose run --user root --rm snake bash

# TODO: don't hardcode region
.PHONY: prod-shell
prod-shell:
	docker run --rm -it --entrypoint bash us-west1-docker.pkg.dev/$(GCP_PROJECT_NAME)/conesnake/conesnake:latest-app

.PHONY: gcloud-config-docker
gcloud-config-docker:
	gcloud auth configure-docker us-west1-docker.pkg.dev

.PHONY: regcred-secret
regcred-secret:
	docker compose run --rm \
		-v "$(HOME)/.kube:/home/conesnake/.kube" \
		snake bash -c '\
	kubectl create namespace conesnake || true \
	&& kubectl delete secret --namespace conesnake --ignore-not-found regcred \
	&& kubectl --namespace conesnake create secret docker-registry regcred \
		--docker-server=https://us-west1-docker.pkg.dev \
		--docker-email=$(GCP_REGISTRY_SA_EMAIL) \
		--docker-username=_json_key \
		--docker-password='\''$(shell cat ./service_key.json)'\'' \
	'

.PHONY: prod-build
prod-build:
	docker compose run --rm snake cargo build --release
	DOCKER_BUILDKIT=1 docker build \
		--target prod \
		--tag us-west1-docker.pkg.dev/$(GCP_PROJECT_NAME)/conesnake/conesnake:latest-app .

	docker push us-west1-docker.pkg.dev/$(GCP_PROJECT_NAME)/conesnake/conesnake:latest-app

# Misc

.PHONY: clean
clean:
	rm -f \
		*.log \
		.k8s/manifest.yaml \
		*.perf \
		perf.data \
		perf.data.* \
		*.png \
		*.out

.PHONY: veryclean
veryclean: clean
	rm -rf target* build* \
		.cargo/registry \
		.cargo/.package-cache \
		.cargo/.package-cache-mutate \
		terraform/.terraform

# Cargo

.PHONY: lint
lint:
	docker compose run --rm snake cargo clippy

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
profile: record report

.PHONY: record
record:
	docker compose run --rm snake bash -c ' \
		cargo build --profile=release-with-debug \
		&& perf record --call-graph dwarf -F 1000 \
			./target-snake/release-with-debug/performance'

.PHONY: report
report:
	docker compose run --rm snake \
		perf report \
			--stdio \
			--stdio-color \
			--percent-limit 0.5 \
			--show-nr-samples \
			--show-cpu-utilization \
			--call-graph srcline

.PHONY: bench
bench:
	docker compose run --rm snake bash -c ' \
		cargo build --release \
		&& ./target-snake/release/performance --num-threads 8'

.PHONY: compare
compare:
	docker compose run --rm snake bash -c ' \
		cargo build --release \
		&& python3 -u ./scripts/play_games.py --mode compare 2>&1 | tee compare.log'

.PHONY: optimize
optimize:
	docker compose run --rm snake bash -c ' \
		cargo build --release \
		&& python3 -u ./scripts/play_games.py --mode optimize 2>&1 | tee optimize.log'

ASM_FUNC ?= "search_worker"

.PHONY: asm
asm:
	docker compose run --rm snake \
		cargo asm --lib $(ASM_FUNC)

# helm

.PHONY: helm-upgrade
helm-upgrade:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/conesnake/.kube" snake \
	helm upgrade prod conesnake \
		--install \
		--create-namespace \
		--namespace conesnake \
		--values conesnake/values-secrets.yaml

.PHONY: helm-uninstall
helm-uninstall:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/conesnake/.kube" snake \
	helm uninstall prod --namespace conesnake

.PHONY: helm-template
helm-template:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/conesnake/.kube" snake \
	helm template \
		--values conesnake/values-secrets.yaml \
		--debug prod conesnake > k8s/manifest.yaml

.PHONY: helm-lint
helm-lint:
	docker compose run --rm --workdir /app/k8s -v "$(HOME)/.kube:/home/conesnake/.kube" snake \
	helm lint conesnake \
		--values conesnake/values-secrets.yaml \

# terraform

.PHONY: terraform-apply
terraform-apply:
	docker compose run --rm -w /app/terraform \
		-v "$(HOME)/.config/gcloud:/home/conesnake/.config/gcloud" \
		-v "$(HOME)/.ssh:/home/conesnake/.ssh" \
		-v "$(HOME)/.kube:/home/conesnake/.kube" \
		snake \
	terraform apply

.PHONY: terraform-init
terraform-init:
	docker compose run --rm \
		-v "$(HOME)/.config/gcloud:/home/conesnake/.config/gcloud" \
		snake \
	terraform -chdir=terraform init -upgrade -migrate-state

.PHONY: terraform-destroy
terraform-destroy:
	docker compose run --rm \
		-v "$(HOME)/.config/gcloud:/home/conesnake/.config/gcloud" \
		-v "$(HOME)/.ssh:/home/conesnake/.ssh" \
		-v "$(HOME)/.kube:/home/conesnake/.kube" \
		snake \
	terraform -chdir=terraform destroy

.PHONY: terraform-output-secret
terraform-output-secret:
	docker compose run --rm \
		-v "$(HOME)/.config/gcloud:/home/conesnake/.config/gcloud" \
		-v "$(HOME)/.ssh:/home/conesnake/.ssh" \
		-v "$(HOME)/.kube:/home/conesnake/.kube" \
		snake \
	terraform -chdir=terraform output -raw conesnake_secret_access_key | base64 --decode | gpg --decrypt

.PHONY: terraform-destroy-wg
terraform-destroy-wg:
	docker compose run --rm \
		-v "$(HOME)/.config/gcloud:/home/conesnake/.config/gcloud" \
		-v "$(HOME)/.ssh:/home/conesnake/.ssh" \
		-v "$(HOME)/.kube:/home/conesnake/.kube" \
		snake \
	terraform -chdir=terraform destroy -target module.k3s_mesh
