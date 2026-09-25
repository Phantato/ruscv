
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

# RUSTSBI_BIN is injected by the Nix devShell (via direnv + flake.nix).
# Fall back to the legacy path only when running outside the devShell.
RUSTSBI_BIN       ?= rustsbi/rustsbi-qemu

QEMU_SERIAL_PORT = 1235
GDB_PORT         = 1234
QEMU_SERIAL = -serial telnet::${QEMU_SERIAL_PORT},server
QEMU_CMD    = qemu-system-riscv64 -M virt --nographic \
	-cpu rv64 -smp 1 -net none 								\
	-bios ${RUSTSBI_BIN} 									\
	${QEMU_SERIAL}
QEMU_LOADER = -device loader,file=${KERNEL_BIN},addr=0x80200000


include user/Makefile
include kernel/Makefile

.PHONY: kernel clean qemu debug lldb clippy readelf objdump nm

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

qemu-debug: QEMU_SERIAL = -serial telnet::${QEMU_SERIAL_PORT},server,nowait
qemu-debug: $(KERNEL_BIN)
	$(call color_header, "Launching QEMU Debugging")
	# Serial MUST be `server,nowait`: the default wait-mode telnet chardev
	# blocks QEMU's main loop until a serial client connects, which also
	# prevents the gdbstub socket from ever binding. With -S + nowait the
	# guest pauses at the reset vector (pc=0x1000) while the gdb port is
	# already listening, so lldb can attach before anything runs.
	$(QEMU_CMD) $(QEMU_LOADER) -s -S -no-shutdown

##------------------------------------------------------------------------------
## Launch QEMU + LLDB in a tmux session (live serial, single command)
##------------------------------------------------------------------------------
debug: QEMU_SERIAL = -serial telnet::${QEMU_SERIAL_PORT},server,nowait
debug: $(KERNEL_BIN)
	$(call color_header, "Launching QEMU + LLDB (tmux)")
	@tmux kill-session -t ruscv-qemu 2>/dev/null; \
	tmux kill-session -t ruscv-debug 2>/dev/null; \
	tmux new-session -d -s ruscv-qemu "$(QEMU_CMD) $(QEMU_LOADER) -s -S -no-shutdown"; \
	tries=0; \
	until nc -z localhost $(GDB_PORT) 2>/dev/null; do \
		sleep 0.2; tries=$$((tries+1)); \
		if [ $$tries -ge 50 ]; then \
			echo "QEMU did not open gdb port $(GDB_PORT); aborting."; \
			tmux kill-session -t ruscv-qemu 2>/dev/null; \
			exit 1; \
		fi; \
	done; \
	tmux new-session -d -s ruscv-debug "lldb -o 'gdb-remote $(GDB_PORT)' $(KERNEL_ELF)"; \
	tmux split-window -t ruscv-debug -v "nc localhost $(QEMU_SERIAL_PORT)"; \
	tmux attach-session -t ruscv-debug; \
	tmux kill-session -t ruscv-qemu 2>/dev/null; \
	tmux kill-session -t ruscv-debug 2>/dev/null; \
	true

##------------------------------------------------------------------------------
## Run the kernel in QEMU
##------------------------------------------------------------------------------
serial:
	nc localhost $(QEMU_SERIAL_PORT)

##------------------------------------------------------------------------------
## Attach lldb debugger
##------------------------------------------------------------------------------
lldb:
	$(call color_header, "Launching LLDB")
	lldb -o "gdb-remote $(GDB_PORT)" ${KERNEL_ELF} 

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

dtb:
	qemu-system-riscv64 -M virt,dumpdtb=dump.dtb
	dtc -o dump.dts dump.dtb

