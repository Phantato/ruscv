## SPDX-License-Identifier: MIT OR Apache-2.0
##
## Copyright (c) 2018-2022 Andre Richter <andre.o.richter@gmail.com>

##--------------------------------------------------------------------------------------------------
## Optional, user-provided configuration values
##--------------------------------------------------------------------------------------------------

include utils.mk

TARGET = riscv64gc-unknown-none-elf
##--------------------------------------------------------------------------------------------------
## BSP-specific configuration values
##--------------------------------------------------------------------------------------------------


##--------------------------------------------------------------------------------------------------
## Targets and Prerequisites
##--------------------------------------------------------------------------------------------------
KERNEL_DIR        = kernel
KERNEL_MANIFEST   = ${KERNEL_DIR}/Cargo.toml
KERNEL_ELF        = target/$(TARGET)/release/kernel
KERNEL_BIN        = kernel.bin
RUSTSBI_BIN       = rustsbi/rustsbi-qemu
LAST_BUILD_CONFIG = target/build_config
# This parses cargo's dep-info file.
# https://doc.rust-lang.org/cargo/guide/build-cache.html#dep-info-files
KERNEL_ELF_DEPS = $(filter-out %: ,$(file < $(KERNEL_ELF).d)) $(KERNEL_MANIFEST) $(LAST_BUILD_CONFIG)

##--------------------------------------------------------------------------------------------------
## Command building blocks
##--------------------------------------------------------------------------------------------------
CFLAG = \
	-C link-arg=-T${KERNEL_DIR}/src/kernel.ld \
	-C force-frame-pointers=yes

RUSTFLAGS_PEDANTIC = ${CFLAG}     \
	-D warnings

COMPILER_ARGS = --target=$(TARGET) --release

RUSTC_CMD   = cargo rustc $(COMPILER_ARGS) -p ruscv_kernel
DOC_CMD     = cargo doc $(COMPILER_ARGS)
CLIPPY_CMD  = cargo clippy $(COMPILER_ARGS)
OBJCOPY_CMD = rust-objcopy \
    --strip-all            \
    -O binary

OBJDUMP_BINARY = llvm-objdump
NM_BINARY      = llvm-nm
READELF_BINARY = llvm-readelf

QEMU_CMD    = qemu-system-riscv64 -M virt -cpu rv64 -s -S --nographic -bios ${RUSTSBI_BIN}
QEMU_LOADER = -device loader,file=${KERNEL_BIN},addr=0x80200000


##--------------------------------------------------------------------------------------------------
## Targets
##--------------------------------------------------------------------------------------------------
.PHONY: all doc qemu miniterm clippy clean readelf objdump nm check

all: $(KERNEL_BIN)

##------------------------------------------------------------------------------
## Save the configuration as a file, so make understands if it changed.
##------------------------------------------------------------------------------
$(LAST_BUILD_CONFIG):
	@rm -f ${LAST_BUILD_CONFIG}
	@mkdir -p target
	@touch $(LAST_BUILD_CONFIG)

##------------------------------------------------------------------------------
## Compile the kernel ELF
##------------------------------------------------------------------------------
$(KERNEL_ELF): $(KERNEL_ELF_DEPS)
	$(call color_header, "Compiling kernel ELF")
	@RUSTFLAGS="$(RUSTFLAGS_PEDANTIC)" $(RUSTC_CMD)

##------------------------------------------------------------------------------
## Generate the stripped kernel binary
##------------------------------------------------------------------------------
$(KERNEL_BIN): $(KERNEL_ELF)
	$(call color_header, "Generating stripped binary")
	@$(OBJCOPY_CMD) $(KERNEL_ELF) $(KERNEL_BIN)
	$(call color_progress_prefix, "Name")
	@echo $(KERNEL_BIN)
	$(call color_progress_prefix, "Size")
	$(call disk_usage_KiB, $(KERNEL_BIN))

##------------------------------------------------------------------------------
## Generate the documentation
##------------------------------------------------------------------------------
doc:
	$(call color_header, "Generating docs")
	@$(DOC_CMD) --document-private-items --open

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
## Clean
##------------------------------------------------------------------------------
clean:
	rm -rf target $(KERNEL_BIN)

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
