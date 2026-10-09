---
version: 1
slug: "src-components-newagent-svelte"
primary_target: "src/components/NewAgent.svelte"
related_targets: ["src/components/LaunchSettings.svelte"]
---

# New agent launch settings

Mode: Operate. This is a local extension of the existing New agent panel, authorized by the owner's accepted plan and instruction to build it.

## Direction contract

THESIS: Pick the program, then choose how this desk starts. Live provider catalogs keep models and effort levels current.

OWN-WORLD: Preserve the office's graphite panel, Geist typography, existing fields, 16px spacing and quiet controls. No new visual system or raster assets.

STORY: The developer selects Claude or Codex, inherits the CLI configuration or chooses a model and permission mode, then starts the task. Advanced controls disclose provider-specific overrides.

FIRST VIEWPORT: Existing 380px right panel. Program picker, Model and Effort, Permissions, task and folder. Advanced is collapsed. The existing sticky Start agent footer remains reachable. At narrow widths the panel fills the available window.

FORM: Extend the incumbent form directly. Signature interaction: switching providers restores that provider's draft; refreshing the catalog leaves the selected model intact. Seed: not applicable. The local-extension branch of `reference/new-work.md` forbids a concept roll and requires inheriting the existing composition. The incumbent DESIGN.md and NewAgent.svelte are the quality reference; no catalog QUALITY BAR card or decision comp applies.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Planned panel (before implementation)

    New agent
    Who should take it? [ Claude Code | Codex | ... ]
    Model                      [Refresh]
    [ Use CLI settings / available model / Custom model... ]
    Effort [ Use CLI settings / model-supported levels ]
    Permissions [ Use CLI settings / provider modes ]
    What should they do? [ task ]
    Folder [ path ] [Browse]
    [ ] Work on a separate copy
    > Advanced launch settings
    Name [ optional ]
    [Start agent] [Cancel]                 ctrl enter

No hardcoded production model list. Preserve provider drafts across refresh, panel close and provider changes. Persist started agents' settings across agent restart, full app restart and duplication. Catalog failures must leave inherited settings and custom IDs usable.

## Finish record

The independent finish review found no material visual fixes. Its contract-evidence finding was resolved by documenting the local-extension branch and the persistence scope; the verdict pass returned ship. Desktop Claude, desktop Codex and narrow-window captures are valid. The documenter confirmed this ordinary extension preserves DESIGN.md and `.impeccable/design.json`; no shipping raster was added.

Verification covers new model and effort arrival after refresh, provider drafts, saved settings, duplication, custom IDs, discovery failure, narrow layout, app restart, bounded/coalesced caches and live catalogs from both installed CLIs. Ctrl+Enter commits the currently edited multiline folders and tool rules before launch. Frontend checks, existing unit/UI checks, Rust tests and Clippy passed.
