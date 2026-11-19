# Ten project colors

Added November 18, 2026 in development.

Five colors join cyan, lime, magenta, violet and mint: blue (`#7aa2ff`), gold (`#f2cc5b`), orchid (`#de8af5`), green (`#8fd46a`) and slate (`#a7b1c6`). Hues were chosen away from the existing five and from orange/red, which stay reserved for urgency and errors. Each has CSS color and RGB tokens, so project dots, tabs, calendar fills and borders work unchanged. Task tags and AI Inbox project proposals offer the same palette because they share the list.

New project now starts on the first color no existing project uses, instead of always cyan. Editing a project keeps its color.

Existing projects keep their colors. No database migration is required (color is free text validated against the native list).

Verification: `pnpm check` 0 errors/warnings; `pnpm test` 198 passed; Rust tests 116 passed (3 ignored). The project edit test now saves orchid. In the dev app (`pnpm dev:seed`, dev identifier) New project showed all ten colors in two rows, preselected mint (the one color the fixture does not use), and creating "Color test" in orchid saved and showed an orchid tab. The personal installation and personal data were not touched; the change is pending a future authorized installation.
