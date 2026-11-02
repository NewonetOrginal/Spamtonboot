target = target/x86_64-unknown-uefi/debug/spamtonboot.efi
efi_boot = build/efi/boot/bootx64.efi
ovmf_fd = $(shell nix-build '<nixpkgs>' -A OVMF.fd --no-out-link)/FV/OVMF.fd

.PHONY: run build

build:
	cargo build --target x86_64-unknown-uefi
	mkdir -p build/efi/boot
	cp $(target) $(efi_boot)

run: build
	qemu-system-x86_64 \
		-bios $(ovmf_fd) \
		-drive file=fat:rw:build,format=raw \
		-net none
