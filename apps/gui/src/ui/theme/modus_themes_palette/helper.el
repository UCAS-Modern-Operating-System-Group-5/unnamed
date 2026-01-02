;;; helper.el --- Generate modus theme color palette definitions in Rust -*- lexical-binding: t -*-

(defun mk/hex-to-rgb (hex)
    "Convert a hex string #RRGGBB to a list of integers (R G B)."
    (let ((r (string-to-number (substring hex 1 3) 16))
             (g (string-to-number (substring hex 3 5) 16))
             (b (string-to-number (substring hex 5 7) 16)))
        (list r g b)))

(defun mk/format-rust-color (name hex)
    "Format a name and hex string into a Rust const definition."
    (let* ((rgb (mk/hex-to-rgb hex))
              (r (nth 0 rgb))
              (g (nth 1 rgb))
              (b (nth 2 rgb))
              ;; Convert lisp-case (blue-warmer) to SCREAMING_SNAKE_CASE (BLUE_WARMER)
              (const-name (upcase (replace-regexp-in-string "-" "_" (symbol-name name)))))
        (format "pub const %s: Color32 = Color32::from_rgb(%d, %d, %d); // %s\n"
            const-name r g b hex)))

(defun mk/convert-modus-to-rust (palette out-file-path)
    "Convert modus-themes-operandi-palette to Rust code in a temporary buffer."
    (interactive)
    (let ((output-buffer (get-buffer-create "*Modus to Rust*"))
             (palette-value (symbol-value palette)))
        (with-current-buffer output-buffer
            (erase-buffer)
            (insert "// Generated from " (symbol-name palette) "\n")
            (insert "use egui::Color32;\n\n")
            
            (dolist (item palette-value)
                (let ((name (car item))
                         (value (cadr item)))
                    (while (and value (symbolp value))
                        (setq value (car (alist-get value palette-value))))
                    (when (and value  ; Can be nil
                              (member name
                                  '(bg-main bg-inactive bg-hover bg-active bg-dim
                                       border bg-blue-subtle fg-mark-select fg-link
                                       fg-main fg-alt err warning info
                                       )))
                        (insert (mk/format-rust-color name value)))))
            
            (write-region (point-min) (point-max) out-file-path))))

;; Run the function
(mk/convert-modus-to-rust 'modus-themes-operandi-palette "./operandi.rs")
(mk/convert-modus-to-rust 'modus-themes-operandi-tinted-palette "./operandi_tinted.rs")
(mk/convert-modus-to-rust 'modus-themes-vivendi-palette "./vivendi.rs")
(mk/convert-modus-to-rust 'modus-themes-vivendi-tinted-palette "./vivendi_tinted.rs")



(provide 'helper)
;;; helper.el ends here
