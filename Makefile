.PHONY: shell
shell:
	docker-compose run --rm snake bash

.PHONY: test
test:
	docker-compose run --rm snake cargo test -- \
		--nocapture \
		--color always --test-threads=1

.PHONY: prod-build
prod-build:
	docker-compose -f docker-compose.prod.yml build

.PHONY: reboot
reboot:
	docker-compose -f docker-compose.prod.yml down -v
	docker-compose -f docker-compose.prod.yml up -d snake

.PHONY: rel-test
rel-test:
	docker-compose run --rm snake cargo test \
		--release -- \
		--nocapture \
		--color always \
		--test-threads=1

.PHONY: profile
profile:
	docker-compose run --rm snake cargo test \
		--profile=release-with-debug \
		--features profile \
		arcade_maze_profile_test -- \
		--nocapture \
		--color always
