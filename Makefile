help: ## Show this help.
	@echo 'usage: make [target] ...'
	@echo
	@echo 'targets:'
	@egrep '^([^:]+)\:[^#]*##\ (.+)' ${MAKEFILE_LIST}

.PHONY: debug
debug: ## Make sure we can debug this.
	cargo b

.PHONY: release
release: ## Make sure we can release this.
	cargo b --release

.PHONY: flash-debug
flash-debug: debug ## Flash the debug firmware.
	cargo espflash

.PHONY: flash
flash: release ## Flash the release firmware.
	cargo espflash --release

.PHONY: monitor
monitor: ## Monitor the device (default).
	cargo espflash --monitor

.PHONY: clean
clean: ## Clean up the build.
	cargo clean

.PHONY: default
default: monitor

