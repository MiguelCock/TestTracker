# Visual baseline — pre-simplify

Captured against the native GPUI build at commit `91c4021` on 2026-11-24, using a synthetic vault
created by the ignored `seed_visual_fixture` test. No client data was used.

- Default viewport: **1440×900** (app default).
- Minimum viewport: **1024×700** (declared minimum).
- Platform: Linux X11, scale factor 1.0.
- Screenshots: local only at `qa-screenshots/pre-simplify-2026-11-24/`, ignored by Git.

## Method

```bash
TESTTRACKER_VISUAL_DIR=/tmp/opencode/tt-visual-data \
  cargo test -p testtracker-core seed_visual_fixture -- --ignored --nocapture

TESTTRACKER_DATA_DIR=/tmp/opencode/tt-visual-data \
  WAYLAND_DISPLAY='' cargo run -p testtracker-app
```

The fixture contains two engagements, three scope assets, two tests (one linked finding, one
in-progress), one standalone finding, one text file, and one generated PDF. Screens were captured
with the window at 1440×900 and again at 1024×700.

## Screen coverage

| Screen / state | 1440×900 | 1024×700 | File |
| --- | --- | --- | --- |
| Unlock | Yes | — | `01-unlock.png`, `50-unlock-fresh.png` |
| Dashboard | Yes | — | `03-dashboard.png` |
| Engagements list | Yes | — | `10-engagements.png` |
| Templates | Yes | Yes | `11-templates.png`, `34-min-templates.png` |
| Settings | Yes | Yes | `12-settings.png`, `35-min-settings.png` |
| Activity | Yes | — | `14-activity.png` |
| MCP bridge | Yes | — | `15-mcp.png` |
| Engagement — Overview | Yes | Yes | `20-overview.png`, `31-min-overview.png` |
| Engagement — Agent | Yes | — | `21-agent.png` |
| Engagement — Tests & Findings | Yes | Yes | `22-tests-findings.png`, `32-min-tests.png` |
| Engagement — Files | Yes | Yes | `23-files.png`, `33-min-files.png` |
| Engagement — Documents | Yes | Yes | `24-documents.png`, `30-min-documents.png` |
| New engagement modal | Yes | Yes | `40-engagement-modal.png`, `41-engagement-modal-min.png` |

Not yet captured (tracked by TT-116): test/finding/file/template modals, forced empty and error
states, import dialogs, plugin dialogs, keyboard-only focus walkthrough, and 125–200% text scaling.

## Defects

| ID | Priority | Screen | Defect | Evidence | Owner |
| --- | --- | --- | --- | --- | --- |
| VB-001 | P0 | Engagement detail, 1024×700 | Header action cluster (status, Export bundle, Edit, Delete) is clipped off the right edge and unreachable. The engagement title also runs past the viewport. | `31-min-overview.png` | TT-112, TT-114 |
| VB-002 | P0 | All detail tabs and global pages, 1024×700 | Main content keeps a width wider than the viewport instead of reflowing. Right-side content is clipped with no horizontal scroll: Overview scope badges, Tests & Findings actions, Files search/actions, Documents actions, Templates header buttons, Settings security card. | `31-min-overview.png`, `32-min-tests.png`, `33-min-files.png`, `30-min-documents.png`, `34-min-templates.png`, `35-min-settings.png` | TT-112, TT-114 |
| VB-003 | P1 | Files | Path is rendered twice. The folder header shows `reports/notes.txt` and the row shows `reports/notes.txt/notes.txt`. The stored relative path appears to include the filename. | `23-files.png`, `33-min-files.png` | TT-030, TT-031 |
| VB-004 | P1 | Templates, 1024×700 | “Import bundle” and “+ Template” are clipped at the right edge; the engagement target chip also overflows. | `34-min-templates.png` | TT-111, TT-112 |
| VB-005 | P1 | Settings, 1024×700 | Two-column profile/security grid does not collapse to one column; the security posture card and its text are clipped. | `35-min-settings.png` | TT-112 |
| VB-006 | P1 | Files, 1024×700 | Search field, “+ Add file”, and row actions (Preview/Download/Delete) are clipped or off-screen. | `33-min-files.png` | TT-112 |
| VB-007 | P2 | Engagement detail | “← All engagements” is horizontally centered over the content instead of left-aligned with the title. | `20-overview.png`, `31-min-overview.png` | TT-111, TT-116 |
| VB-008 | P2 | New engagement modal, 1024×700 | Modal fills the full content height, touching the title bar and bottom edge; no vertical breathing room. | `41-engagement-modal-min.png` | TT-112 |
| VB-009 | P2 | New engagement modal | Subtitle still says “Scope, assumptions, and contacts can be refined from the engagement overview.” Those sections are removed from the target model. | `40-engagement-modal.png`, `41-engagement-modal-min.png` | TT-021 |
| VB-010 | P3 | Engagements list | Empty start/end dates render as “— — —” rather than a single “Not set” value. | `10-engagements.png` | TT-115 |
| VB-011 | P2 | Engagement detail, 1440×900 | Long engagement titles have no truncation or wrap policy; at default width the title nearly collides with the status and action cluster. | `20-overview.png` | TT-111 |

No duplicate GPUI element-ID warnings appeared in the application log during this pass.

## Batch 1 re-check

Batch 1 added viewport metrics, responsive page headers, table/action wrapping, constrained modals, and
an explicit main-content width. Re-checked at 900×600 on 2026-11-24:

| Defect | Result |
| --- | --- |
| VB-001 header action clipping | Fixed; title ellipsizes and actions wrap below |
| VB-002 page-level horizontal clipping | Fixed; main content width is bounded and pages reflow |
| VB-004 Templates header clipping | Fixed; actions wrap |
| VB-005 Settings grid clipping | Fixed; grid collapses to one column |
| VB-006 Files action clipping | Fixed; header and row actions wrap |
| VB-007 centered back link | Fixed; left-aligned with content |
| VB-008 modal edge contact | Fixed; modal keeps a viewport margin and scrolls its body |
| VB-011 long engagement title | Fixed with one-line ellipsis |
| VB-003 duplicated Files path | Still open (E3 data-path contract) |
| VB-009 stale engagement modal copy | Still open (E2 copy cleanup) |
| VB-010 empty-date formatting | Still open (TT-115 state polish) |

New evidence is stored under `qa-screenshots/` (ignored). TT-114 should confirm the same results at
1024×700 and 1440×900 before closing.

## Notes for TT-114

- VB-001 and VB-002 likely share one root cause: a fixed minimum width on the main content rather
  than a responsive layout. Fix the container first, then re-check every screen.
- VB-003 is a data-path contract issue for E3, not a cosmetic fix. Confirm whether
  `engagement_files.relative_path` should be a folder (`reports`) or the full path including the
  filename before changing the renderer.
- VB-009 is a copy fix in E2; do not keep references to risks, assumptions, or contacts in any
  dialog once those fields are removed.
- Re-run this matrix at 1024×700 after TT-112 and attach the new screenshots to the issue.
