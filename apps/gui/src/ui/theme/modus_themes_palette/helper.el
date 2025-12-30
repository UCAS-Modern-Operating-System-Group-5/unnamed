;;; helper.el --- Generate modus theme color palette definitions in Rust -*- lexical-binding: t -*-

(defun christina/hex-to-rgb (hex)
    "Convert a hex string #RRGGBB to a list of integers (R G B)."
    (let ((r (string-to-number (substring hex 1 3) 16))
             (g (string-to-number (substring hex 3 5) 16))
             (b (string-to-number (substring hex 5 7) 16)))
        (list r g b)))

(defun christina/format-rust-color (name hex)
    "Format a name and hex string into a Rust const definition."
    (let* ((rgb (christina/hex-to-rgb hex))
              (r (nth 0 rgb))
              (g (nth 1 rgb))
              (b (nth 2 rgb))
              ;; Convert lisp-case (blue-warmer) to SCREAMING_SNAKE_CASE (BLUE_WARMER)
              (const-name (upcase (replace-regexp-in-string "-" "_" (symbol-name name)))))
        (format "pub const %s: Color32 = Color32::from_rgb(%d, %d, %d); // %s\n"
            const-name r g b hex)))

(defun christina/convert-modus-to-rust (palette)
    "Convert modus-themes-operandi-palette to Rust code in a temporary buffer."
    (interactive)
    (let ((output-buffer (get-buffer-create "*Modus to Rust*")))
        (with-current-buffer output-buffer
            (erase-buffer)
            (insert "// Generated from " (symbol-name palette) "\n")
            (insert "use egui::Color32;\n\n")
            
            (dolist (item (symbol-value palette))
                (let ((name (car item))
                         (value (cadr item)))
                    ;; (when (stringp value)
                    ;;     (insert (christina/format-rust-color name value)))

                    (when (member name
                              '(bg-main bg-inactive bg-hover bg-active bg-dim
                                   border bg-blue-subtle cyan blue-warmer
                                   yellow-warmer red
                                   fg-main fg-alt))
                        (insert (christina/format-rust-color name value)))))
            
            ;; Optional: Display the buffer
            (switch-to-buffer output-buffer)
            (rust-ts-mode))))

;; Run the function
(christina/convert-modus-to-rust 'modus-themes-operandi-palette)
(christina/convert-modus-to-rust 'modus-themes-operandi-tinted-palette)
(christina/convert-modus-to-rust 'modus-themes-vivendi-palette)
(christina/convert-modus-to-rust 'modus-themes-vivendi-tinted-palette)



(provide 'helper)
;;; helper.el ends here
