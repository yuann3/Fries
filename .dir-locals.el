((nil . ((fill-column . 100)                          ;; line length guideline
         (projectile-project-run-cmd . "cargo run")  ;; how to run the project
         (projectile-project-compilation-cmd . "cargo build") ;; how to compile
         (projectile-project-test-cmd . "cargo test") ;; how to test
         (eval . (setq-local indent-tabs-mode nil)))) ;; never use tabs

 (rust-mode . ((rust-format-on-save . t)             ;; auto-format on save
               (lsp-rust-analyzer-cargo-watch-command . "clippy") ;; run clippy instead of check
               (lsp-rust-analyzer-proc-macro-enable . t)          ;; enable proc macros
               (lsp-enable-on-type-formatting . nil)              ;; avoid laggy typing
               )))
