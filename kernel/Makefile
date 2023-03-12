## SPDX-License-Identifier: MIT OR Apache-2.0
##
## Copyright (c) 2018-2022 Andre Richter <andre.o.richter@gmail.com>

##--------------------------------------------------------------------------------------------------
## Optional, user-provided configuration values
##--------------------------------------------------------------------------------------------------

KERNEL_LOG ?= ERROR
KERNEL_LOG_LEVEL = $(shell echo ${KERNEL_LOG} | tr '[:upper:]' '[:lower:]')

##--------------------------------------------------------------------------------------------------
## Targets and Prerequisites
##--------------------------------------------------------------------------------------------------
KERNEL_DIR        = kernel
KERNEL_MANIFEST   = ${KERNEL_DIR}/Cargo.toml
KERNEL_ELF        = $(TARGET_DIR)/kernel
KERNEL_BIN        = kernel.bin
LAST_BUILD_CONFIG = target/build_config
# This parses cargo's dep-info file.
# https://doc.rust-lang.org/cargo/guide/build-cache.html#dep-info-files
KERNEL_ELF_DEPS = $(filter-out %: ,$(file < $(KERNEL_ELF).d)) $(KERNEL_MANIFEST) $(LAST_BUILD_CONFIG) \
	$(USER_BINS)

##--------------------------------------------------------------------------------------------------
## Command building blocks
##--------------------------------------------------------------------------------------------------
KERNEL_LINK_SCRIPT := \
	-C link-arg=-T${KERNEL_DIR}/src/kernel.ld
KERNEL_COMPILE_ARGS := \
	--features log-${KERNEL_LOG_LEVEL}

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
	@RUSTFLAGS="$(COMPILER_ARGS) ${KERNEL_LINK_SCRIPT}" $(RUSTC_CMD) ${KERNEL_COMPILE_ARGS} \
		-p ruscv_kernel

##------------------------------------------------------------------------------
## Generate the stripped kernel binary
##------------------------------------------------------------------------------
$(KERNEL_BIN): $(KERNEL_ELF)
	$(call color_header, "Generating stripped binary")
	@$(OBJCOPY_CMD) $(KERNEL_ELF) $(KERNEL_BIN)
	$(call color_progress_prefix, "Name")
	@echo $(KERNEL_BIN)
	$(call color_progress_prefix, "Size")
	@$(call disk_usage_KiB, $(KERNEL_BIN))
