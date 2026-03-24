TARGET := riscv64gc-unknown-none-elf
MODE ?= debug
KERNEL_BIN := target/$(TARGET)/$(MODE)/os
KERNEL_ENTRY_PA ?= 0x80200000
BOOTLOADER ?= default

RUSTFLAGS := -Clink-arg=-Tsrc/linker.ld -Cforce-frame-pointers=yes

.PHONY: build run clean

build:
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --target $(TARGET)

run: build
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-kernel $(KERNEL_BIN)

clean:
	cargo clean
