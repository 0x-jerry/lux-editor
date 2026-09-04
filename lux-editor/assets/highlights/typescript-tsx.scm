; TSX-only additions to typescript.scm: JSX nodes do not exist in the plain
; TypeScript grammar, so they live here and are concatenated for the tsx
; language only.

(jsx_opening_element
  name: (_) @tag)
(jsx_closing_element
  name: (_) @tag)
(jsx_self_closing_element
  name: (_) @tag)
(jsx_attribute
  (property_identifier) @attribute)
