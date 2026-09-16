.PHONY: check fmt test build pi32 pi64 clean

check:
	docker compose run --rm dev cargo check

fmt:
	docker compose run --rm dev cargo fmt -- --check

test:
	docker compose run --rm dev cargo test

build:
	docker compose run --rm dev cargo build --release

pi32:
	./scripts/cross-build.sh pi32

pi64:
	./scripts/cross-build.sh pi64

clean:
	docker compose down --volumes
	rm -rf target
