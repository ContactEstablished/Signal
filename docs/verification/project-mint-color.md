# Fifth project color — Mint

Added September 17, 2026 in development.

Mint (`#6ee7b7`) joins cyan, lime, magenta and violet in New project and Edit project. Its CSS color and RGB tokens drive the existing project dots, underlines, calendar fills and borders. The shared task-tag palette also exposes Mint; native tag validation now uses the project palette plus its existing orange option so it can save every offered color.

Existing projects keep their colors. No database migration or fixture change is required.

Verification: `pnpm check` passed with zero errors/warnings; `pnpm build` passed with the existing chunk-size advisory; all seven native workspace tests and `cargo fmt -- --check` passed. Existing persistence tests now exercise Mint project creation/editing and tag saving. No native visual verification is claimed. The personal installer and personal data were not touched; the change is pending a future authorized installation.
