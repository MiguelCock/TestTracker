# Information architecture

Status: the target shell is implemented in the native GPUI build. This document is the reference
for E1, E2, E4, E8, and E9, and records which surfaces are current, transitional, or removed.

## Principles

- Engagement work lives in five tabs. Global libraries and vault administration live in the
  sidebar.
- One screen has one primary purpose and one primary action area.
- Panels that duplicate another surface are removed, not hidden behind a second navigation path.
- Every destructive action requires an explicit confirmation and is audited.
- The minimum supported window is 900×600. A screen that cannot fit must scroll inside one
  container, not trap the whole window.

## Global navigation

| Sidebar item | Route | Purpose | Owner |
| --- | --- | --- | --- |
| Dashboard | `Route::Dashboard` | Portfolio counts, recent engagements, quick links | E1 |
| Engagements | `Route::Engagements` | Engagement list, filters, create/open | E1 |
| Templates | `Route::Templates` | Global template and bundle library | E6 |
| MCP | `Route::Mcp` | Local MCP bridge status, token, approvals | E7 |
| Settings | `Route::Settings` | Profile, security, AI, plugins, maintenance, Activity | E1/E8 |

`Route::Activity` is reached from Settings, not from the sidebar. It keeps Settings highlighted as
the active section. The MCP route stays top-level through E7; moving it under Settings is an E9
candidate and must not be done implicitly.

## Engagement tabs

| Tab | Purpose | Primary actions | Owner |
| --- | --- | --- | --- |
| Overview | Engagement metadata, application summary, scope | Edit engagement, edit scope | E2 |
| Agent | Plan/Build chat, plan pane, proposed changes | Switch mode, submit plan, review/apply changes | E5 |
| Tests & Findings | Test rows with linked findings, standalone findings | Add/edit test, add linked finding, filter, reorder | E4 |
| Files | Encrypted virtual file workspace | Upload, preview, search, download, rename/move, delete | E3 |
| Documents | Generated PDF/DOCX/XLSX artifacts | Generate, open, delete | E3 |

Rules:

- Opening an engagement defaults to Overview.
- The tab bar never contains Evidence, Review, Findings, Audit, or Plugins.
- Findings are edited in the same drawer as the linked test, and standalone findings remain in a
  separate section of Tests & Findings.
- Files are addressed by engagement-relative virtual paths, never by arbitrary OS paths.
- The Agent can read only through the shared tool registry; it cannot write except through
  proposals.

## Settings

| Section | Purpose | Owner |
| --- | --- | --- |
| Activity and audit chain | Global event list, filters, chain verification | E8 |
| Maintenance | One-way legacy evidence migration | E3 |
| Local profile | Display name, role, vault state | E1 |
| AI providers | Provider configuration, encrypted keys, policy | E5 |
| MCP bridge | Token, endpoint, pending proposals | E7 |
| Plugins | Publisher trust, install, enable/disable, delete | E1 |

Activity is read-only. Chain verification is a read operation and never rewrites audit rows.

## Templates

The Templates page is global, not engagement-scoped. It contains:

- Search by domain, category, and title.
- Custom template create/edit/delete and duplicate-from-built-in.
- Curated bundle apply, import, export, and delete.
- “Use in engagement” picker.

Built-in templates remain read-only. Imported content is untrusted and is validated before it is
stored.

## Removed surfaces

These surfaces are removed from navigation and must not be reintroduced:

| Removed surface | Replacement | Remaining work |
| --- | --- | --- |
| Risks, assumptions, contacts | Application summary and scope | E2 removes fields, reports, MCP/agent context; E2 drops tables |
| Evidence tab | Files | E3 migrates evidence and removes services/tools |
| Review, snapshots, approvals | Audit events and document history | E3 removes UI/services; E2 drops tables |
| Findings tab | Tests & Findings | E4 removes the route and redirects deep links |
| Audit tab | Settings → Activity | E8 removes the engagement tab and render code |
| Engagement Plugins tab | Settings → Plugins | E1 removes the engagement surface |

The code still contains dead render functions and legacy modal variants for some removed screens
(`render_evidence`, `render_review`, `render_findings`, `render_audit`, `CreateSnapshot`,
`SignOff`, `AddEvidence`). They are unreachable from the current `Route` enum. TT-120 removes
them; do not delete audit or bundle-signing infrastructure with them.

## Navigation and state rules

- Sidebar active state follows the route: an engagement detail keeps Engagements active; Activity
  keeps Settings active.
- Back navigation from Activity returns to Settings.
- Deep links carry the engagement ID and tab. Dashboard links must land on a populated filter or
  tab, not on a removed screen.
- Closing a modal with Escape is equivalent to Cancel, except for destructive confirmations where
  Cancel is explicit.
- Unsaved edits prompt before navigation.
- Loading, empty, and error states are owned by the screen, not by the global shell.

## Modal and drawer registry

| Modal / drawer | Used by | Size rule |
| --- | --- | --- |
| Engagement form | Engagements, Overview | Fits 900×600; scrolls body |
| Scope editor | Overview | Fits 900×600; scrolls body |
| Test case form | Tests & Findings | Side drawer; body scrolls |
| Finding form | Tests & Findings | Side drawer; body scrolls |
| Add file / file preview | Files | Fits 900×600; preview scrolls |
| Generate document | Documents | Fits 900×600 |
| Template form / template picker | Templates, Tests | Fits 900×600 |
| Import assessment / import bundle / export bundle | Templates, Engagements | Fits 900×600 |
| Provider form | Settings → AI | Fits 900×600 |
| Publisher form / plugin result | Settings → Plugins | Fits 900×600 |
| Agent proposal / preview | Agent | Side pane or drawer; changes list scrolls |
| Confirmation dialog | All destructive actions | Short message, two actions |

## Screen ownership

| Screen | Epic / issue |
| --- | --- |
| Dashboard, Engagements, shell navigation | E1 (TT-010 … TT-016) |
| Overview simplification and field removal | E2 (TT-020 … TT-026) |
| Files, evidence migration, document builders | E3 (TT-030 … TT-043) |
| Tests & Findings workbench | E4 (TT-050 … TT-056) |
| Agent Plan/Build | E5 (TT-060 … TT-073) |
| Templates page and bundles | E6 (TT-080 … TT-085) |
| OpenCode integration | E7 (TT-090 … TT-094) |
| Settings → Activity | E8 (TT-100 … TT-103) |
| Primitives, layout, accessibility, visual QA | E9 (TT-110 … TT-116) |
| Dead-code removal and release | E10 (TT-120 … TT-124) |

## Current deltas

- E1: routes and five tabs exist; remaining work is transitional flags, dashboard deep links, and
  consistent page headers/action bars.
- E2: legacy fields still exist in the database, DTOs, reports, and MCP/agent context. Overview
  must show only metadata, summary, and scope.
- E3: Files and evidence migration services exist; legacy evidence UI/services and bundle v1
  behavior still need removal.
- E4: the unified screen exists; confirm linked/standalone findings, shared filters, and removal of
  the old Findings route.
- E8: Settings → Activity exists; audit tab render code and large-history pagination remain.
- E9: visual baseline and the 900×600 pass are tracked in `docs/visual-baseline.md`.
