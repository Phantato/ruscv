USER_APP_DIR := user/src/bin
USER_APPS := $(wildcard $(USER_APP_DIR)/*.rs)
USER_ELFS := $(patsubst $(USER_APP_DIR)/%.rs, $(TARGET_DIR)/%, $(USER_APPS))
USER_BINS := $(patsubst $(USER_APP_DIR)/%.rs, $(TARGET_DIR)/%.bin, $(USER_APPS))

USER_LINK_SCRIPT = -C link-arg=-Tuser/src/linker.ld

.PHONY: user

user: ${USER_BINS}

${USER_ELFS}:
	$(call color_header, "Compiling user ELF")
	@RUSTFLAGS="$(COMPILER_ARGS) $(USER_LINK_SCRIPT)" $(RUSTC_CMD) -p user_lib

${USER_BINS}: ${USER_ELFS}
	$(call color_header, "Generating stripped binary")
	$(foreach elf, $(USER_ELFS), $(OBJCOPY_CMD) $(elf) $(patsubst $(TARGET_DIR)/%, $(TARGET_DIR)/%.bin, $(elf));)
	$(call color_progress_prefix, "Name")
	@echo $(USER_BINS)
	$(call color_progress_prefix, "Size")
	@echo
	@$(foreach bin, $(USER_BINS), $(call disk_usage_KiB, $(bin));)
	