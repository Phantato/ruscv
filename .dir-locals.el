;;; Directory-local configuration for ruscv.
;;;
;;; This file provides project-scoped debugging helpers that run the
;;; kernel in QEMU and attach a debugger from Emacs, without touching
;;; the user's global configuration (`~/.emacs.d').
;;;
;;; It relies on `dape' (a DAP client) driving LLDB's `lldb-dap'
;;; adapter, mirroring `.vscode/launch.json'.  The QEMU side reuses the
;;; repository's existing `make qemu-debug' target (gdbstub on :1234,
;;; paused at the reset vector, serial on :1235).
;;;
;;; Because this block defines functions it is an `eval' form, so Emacs
;;; asks once per session to trust it.  Answer `!' to accept this file.
((nil . ((eval .
          (progn
            (require 'comint)

            (defun ruscv--find-lldb-dap ()
              "Locate the `lldb-dap' adapter on absolute path."
              (or (executable-find "lldb-dap")
                  (car (file-expand-wildcards
                        "/nix/store/*-lldb-*/bin/lldb-dap"))
                  (user-error
                   "Cannot find `lldb-dap'; this repo's dev shell provides it (direnv)")))

            (defun ruscv--port-open-p (host port &optional timeout)
              "Return t if a TCP connection to HOST:PORT can be opened.
Returns nil if the connection is refused or TIMEOUT seconds elapse."
              (condition-case nil
                  (let ((proc (open-network-stream
                               "ruscv-probe" nil host port
                               :type 'plain :nowait nil)))
                    (prog1 t (delete-process proc)))
                (file-error nil)
                (t nil)))

            (defun ruscv--await (host port callback &optional deadline)
              "Awaits HOST:PORT then invokes CALLBACK.
Calls itself on a timer while the port is unreachable.  Gives up
after DEADLINE (float time, default 30s)."
              (if (ruscv--port-open-p host port)
                  (funcall callback)
                (let ((dl (or deadline (+ (float-time) 30.0))))
                  (if (> (float-time) dl)
                      (message "ruscv: gdb port %s:%s never opened" host port)
                    (run-at-time 0.3 nil #'ruscv--await host port callback dl)))))

            (defun ruscv/qemu-debug (&optional halt)
              "Build and boot the kernel in QEMU, then attach the DAP debugger.

Runs `make qemu-debug' in the background (which rebuilds the kernel
and starts QEMU paused at the reset vector, gdbstub on port 1234,
serial console on port 1235), waits for the gdb port to come up, and
finally opens an `lldb-dap' session through `dape'.

By default the guest is resumed right after attaching (`\"c\"', like
the \"Debug Release Kernel\" entry in `.vscode/launch.json'), so the
kernel boots and you break into it whenever you like.  With a prefix
argument HALT is non-nil and the guest stays stopped at its starting
point, letting you drive it step by step from the reset/boot code.

Switching to kernel source after attaching, `dape-breakpoint-toggle',
then `dape-continue' to hit your breakpoint.
See also `ruscv/qemu-serial' and `ruscv/qemu-debug-stop'."
              (interactive "P")
              (ruscv/qemu-silent-kill)
              (async-shell-command "make qemu-debug" "*ruscv-qemu*")
              (let* ((root (or (and (project-current)
                                    (project-root (project-current)))
                               default-directory))
                     (config
                      (list
                       'command (ruscv--find-lldb-dap)
                       'command-cwd root
                       :type "lldb-dap"
                       :request "custom"
                       :targetCreateCommands
                       (vector (format "target create %s"
                                       (expand-file-name
                                        "target/riscv64gc-unknown-none-elf/release/kernel"
                                        root)))
                       :processCreateCommands
                       (if halt
                           (vector "gdb-remote 1234")
                         (vector "gdb-remote 1234" "c"))
                       :sourceLanguages (vector "rust")
                       :cwd root)))
                (ruscv--await
                 "127.0.0.1" 1234
                 (lambda ()
                   (dape config)))))

            (defun ruscv/qemu-serial ()
              "Open the kernel's serial console (QEMU telnet port 1235)."
              (interactive)
              (pop-to-buffer (get-buffer-create "*ruscv-serial*"))
              (unless (comint-check-proc)
                (apply #'make-comint
                       "ruscv-serial"
                       (or (executable-find "nc")
                           (user-error "Cannot find `nc'"))
                       nil '("localhost" "1235"))))

            (defun ruscv/qemu-silent-kill ()
              "Without prompting, kill a running ruscv QEMU and its buffer."
              (when-let* ((proc (get-buffer-process "*ruscv-qemu*")))
                (delete-process proc))
              (when (get-buffer "*ruscv-qemu*")
                (kill-buffer "*ruscv-qemu*"))
              (call-process-shell-command
               "pkill -f qemu-system-riscv64" nil nil nil)
              nil)

            (defun ruscv/qemu-debug-stop ()
              "Terminate QEMU and related buffers started by `ruscv/qemu-debug'."
              (interactive)
              (ruscv/qemu-silent-kill)
              (when-let* ((proc (get-buffer-process "*ruscv-serial*")))
                (delete-process proc))
              (message "ruscv: QEMU stopped")))))))