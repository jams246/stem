CARGO ?= cargo
TARGET ?= x86_64-unknown-linux-gnu
BIN := target/$(TARGET)/production/stem

.PHONY: help build\:production

help:
	@echo "make build:production  build an optimized $(TARGET) release into $(BIN)"

build\:production:
	$(CARGO) build --locked --profile production --target $(TARGET)
	@echo "$(BIN)"
