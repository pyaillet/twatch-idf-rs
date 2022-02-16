DEVICE=/dev/cu.usbserial-0236B9E6

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
	espflash $(DEVICE) target/xtensa-esp32-espidf/debug/twatch-idf-rs

.PHONY: flash
flash: release ## Flash the release firmware.
	espflash $(DEVICE) target/xtensa-esp32-espidf/release/twatch-idf-rs

.PHONY: monitor
monitor: flash-debug ## Monitor the device (default).
	espmonitor $(DEVICE)

.PHONY: clean
clean: ## Clean up the build.
	cargo clean

.PHONY: default
default: monitor

