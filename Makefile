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

.PHONY: prod-shell
prod-shell:
	docker run --rm -it --entrypoint bash \
		us-west1-docker.pkg.dev/$(shell gcloud config get-value project)/conesnake/conesnake:latest-app

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
		--docker-email=$(shell gcloud iam service-accounts list \
			--filter=displayName="conesnake_registry_service_account" --format="get(email)" 2>/dev/null) \ 
		--docker-username=_json_key \
		--docker-password='\''$(shell cat ./service_key.json)'\'' \
	'

.PHONY: prod-build
prod-build:
	docker compose run --rm snake cargo build --release
	DOCKER_BUILDKIT=1 docker build \
		--target prod \
		--tag us-west1-docker.pkg.dev/$(shell gcloud config get-value project)/conesnake/conesnake:latest-app .

	docker push us-west1-docker.pkg.dev/$(shell gcloud config get-value project)/conesnake/conesnake:latest-app

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
	( chmod 700 -R .go || true ) && \
	rm -rf target* build* \
		.go \
		.cargo/registry \
		.cargo/.package-cache \
		.cargo/.package-cache-mutate \
		terraform/.terraform

# Cargo

.PHONY: lint
lint:
	docker compose run --rm snake cargo clippy

.PHONY: test
test: move
	docker compose run --rm snake cargo test -- \
		--nocapture \
		--color always

.PHONY: rel-test
rel-test: move
	docker compose run --rm snake cargo test \
		--release -- \
		--nocapture \
		--color always \
		--test-threads=1

.PHONY: move
move .go/bin/move:
	docker compose run --rm snake bash -c ' \
		cd rules && go install'

# Profiling

.PHONY: profile
profile: build-profile record report

.PHONY: profile-mem
profile-mem: build-profile record-mem report

.PHONY: build-profile
build-profile:
	docker compose run --rm snake bash -c ' \
		RUSTFLAGS="-C force-frame-pointers=yes" cargo build \
			-Z build-std \
			--profile=release-with-debug \
			--target x86_64-unknown-linux-gnu'

.PHONY: record
record:
	docker compose run --rm snake \
		perf record \
			--call-graph dwarf \
			-e cycles \
			-F 1000 \
			./target-snake/x86_64-unknown-linux-gnu/release-with-debug/benchmark

.PHONY: report
report:
	docker compose run --rm snake \
		perf report \
			--stdio \
			--stdio-color \
			--percent-limit 3 \
			--show-nr-samples \
			--show-cpu-utilization \
			--call-graph srcline

.PHONY: record-mem
record-mem:
	docker compose run --rm snake \
		perf record \
			--call-graph dwarf \
			-e cache-misses \
			-F 1000 \
			./target-snake/x86_64-unknown-linux-gnu/release-with-debug/benchmark

.PHONY: stat
stat:
	docker compose run --rm snake bash -c '\
		cargo build --release \
		&& perf stat \
			-e task-clock,cycles,instructions,branches,branch-misses \
			-e cache-references,cache-misses \
			./target-snake/release/benchmark'

# Performance

.PHONY: performance
performance:
	docker compose run --rm snake bash -c ' \
		cargo build --release \
		&& ./target-snake/release/performance --num-threads 8'

.PHONY: bench
bench:
	docker compose run --rm snake bash -c ' \
		cargo build --release \
		&& ./target-snake/release/benchmark'

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

ASM_FUNC ?= "conesnake::search::NodeState::duct_scores_simd"

.PHONY: asm
asm:
	docker compose run --rm snake bash -ic '\
		cargo asm --lib --rust --color $(ASM_FUNC) | less -R'

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
