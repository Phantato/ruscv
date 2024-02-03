
include utils.mk

TARGET = riscv64gc-unknown-none-elf
TARGET_DIR := target/$(TARGET)/release

COMPILER_ARGS = -C force-frame-pointers=yes \
#	-D warnings
COMPILER_FLAG := --target=$(TARGET) --release

RUSTC_CMD   = cargo rustc $(COMPILER_FLAG)
DOC_CMD     = cargo doc $(COMPILER_FLAG)
CLIPPY_CMD  = cargo clippy $(COMPILER_FLAG)
OBJCOPY_CMD = rust-objcopy \
    --strip-all            \
    -O binary

OBJDUMP_BINARY = llvm-objdump
NM_BINARY      = llvm-nm
READELF_BINARY = llvm-readelf

RUSTSBI_BIN       = rustsbi/rustsbi-qemu
QEMU_CMD    = qemu-system-riscv64 -M virt -s -S --nographic \
	-cpu rv64 -smp 1 -net none 								\
	-bios ${RUSTSBI_BIN} 									\
	-serial telnet::1235,server
QEMU_LOADER = -device loader,file=${KERNEL_BIN},addr=0x80200000


include user/Makefile
include kernel/Makefile

.PHONY: kernel clean qemu lldb clippy readelf objdump nm

kernel: ${KERNEL_BIN}

##------------------------------------------------------------------------------
## Clean
##------------------------------------------------------------------------------
clean:
	rm -rf target $(KERNEL_BIN)

##------------------------------------------------------------------------------
## Run the kernel in QEMU
##------------------------------------------------------------------------------
qemu: $(KERNEL_BIN)
	$(call color_header, "Launching QEMU")
	$(QEMU_CMD) $(QEMU_LOADER)

##------------------------------------------------------------------------------
## Attach lldb debugger
##------------------------------------------------------------------------------
lldb: qemu
	$(call color_header, "Launching LLDB")
	lldb -o "gdb-remote 1234" ${KERNEL_ELF} 

##------------------------------------------------------------------------------
## Run clippy
##------------------------------------------------------------------------------
clippy:
	@RUSTFLAGS="$(RUSTFLAGS_PEDANTIC)" $(CLIPPY_CMD)

##------------------------------------------------------------------------------
## Run readelf
##------------------------------------------------------------------------------
readelf: $(KERNEL_ELF)
	$(call color_header, "Launching readelf")
	@$(READELF_BINARY) --headers $(KERNEL_ELF)

##------------------------------------------------------------------------------
## Run objdump
##------------------------------------------------------------------------------
objdump: $(KERNEL_ELF)
	$(call color_header, "Launching objdump")
	@$(OBJDUMP_BINARY) --disassemble --demangle \
                --section .text   \
                --section .rodata \
                $(KERNEL_ELF) | rustfilt

##------------------------------------------------------------------------------
## Run nm
##------------------------------------------------------------------------------
nm: $(KERNEL_ELF)
	$(call color_header, "Launching nm")
	@$(NM_BINARY) --demangle --print-size $(KERNEL_ELF) | sort | rustfilt
