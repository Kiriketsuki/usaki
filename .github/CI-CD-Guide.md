# CI/CD Guide — kTemplate

## Overview

Year-prefixed semver (`YY.Major.Minor.Patch[hotfix]`) with automated version bumping, issue-to-branch automation, and GitHub release management.

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `version-validation.yml` | PR to `main` or `release` | Validate `VERSION` file format and uniqueness vs latest tag |
| `version-bump.yml` | PR merged to `main` / direct push with `hotfix:` | Auto-increment version in `VERSION` file |
| `manual-version-bump.yml` | `workflow_dispatch` | Manual bump (major/minor/patch) or year rollover |
| `release.yml` | PR merged to `release` / push to `release` | Sync VERSION from main, create Git tag + GitHub Release |
| `issue-branch-handler.yml` | Issue labeled `epic`, `feature`, `task`, `bug`, or `hotfix` | Create branch + draft PR + sub-issue parent tracking |
| `deploy-docs.yml` | Push to `main` touching `docs/visual-explainer/**`, or `workflow_dispatch` | Build gallery index and push to `visual-explainer` branch for GitHub Pages |

## VERSION File

Located at the repository root. Single source of truth for the version.

```text
26.0.0.0
```

**Format**: `YY.Major.Minor.Patch[Suffix]`
- YY = two-digit year
- Major = epics / large work units (resets Minor, Patch on increment)
- Minor = features (resets Patch on increment)
- Patch = tasks and bug fixes
- Suffix = emergency hotfix letters (a, b, … z, aa, ab …) — rare

## Branch Strategy

```
main              ← protected
release           ← production releases
visual-explainer  ← GitHub Pages (managed by CI, do not commit to directly)
epic/{n}-...      ← large work units (branch from main)
hotfix/{n}-…      ← emergency fixes (branch from main)
bug/{n}-…         ← bug fixes (branch from main)
feature/{n}-…     ← features (branch from parent issue's branch, or main)
task/{n}-…        ← tasks (branch from parent issue's branch, or main)
```

Base-branch hierarchy:
- `epic`, `hotfix`, `bug` → `main`
- `feature`, `task` → parent issue's branch when the issue is a sub-issue, else `main`

Sub-issues automatically branch from their parent issue's branch via `issue-branch-handler.yml`. A per-issue `Branch off:` override can target any existing remote branch (see below).

## Version Bumping Rules

| Branch type merged → main | Bump |
|---------------------------|------|
| `epic/*` | Major +1, Minor → 0, Patch → 0 |
| `feature/*` | Minor +1, Patch → 0 |
| `task/*` or `bug/*` | Patch +1 |
| `hotfix/*` or direct push with `hotfix:` in message | Suffix (a, b, … z, aa, ab …) |

On year change, the next bump resets to `YY.0.0.0` regardless of bump type.

## Issue Hierarchy

Issues are labeled with one of five types: `epic`, `feature`, `task`, `bug`, or `hotfix`.
The label drives both the branch prefix and the base branch the handler picks.

```
Epic (epic label)
└── Feature (feature label, sub-issue of Epic)
    └── Task (task label, sub-issue of Feature)
```

Bugs and hotfixes are standalone and always branch from `main`.

- Create an Epic → branch `epic/{n}-...` auto-created from `main`, draft PR opened
- Add Feature as sub-issue of Epic → branch `feature/{n}-...` auto-created from the epic branch
- Add Task as sub-issue of Feature → branch `task/{n}-...` auto-created from the feature branch
- Create a Bug → branch `bug/{n}-...` auto-created from `main`
- Create a Hotfix → branch `hotfix/{n}-...` auto-created from `main`

`feature` and `task` branch from their parent issue's branch when sub-issue tracking is
set; otherwise they branch from `main`. `epic`, `bug`, and `hotfix` always branch from `main`.

## Per-Issue Base Branch Override

By default the issue handler picks the base branch from the branch-type
hierarchy (`epic`/`hotfix`/`bug` → `main`, `feature`/`task` → parent
issue's branch when sub-issue tracking is set, else `main`).

You can override that on a per-issue basis with a **`Branch off:`** field.
Each issue template includes the field with `main` prefilled — change it
to any existing remote branch name to make the auto-created branch and
draft PR target that branch instead.

```
Branch off: feature/149-new-trip-flow
```

This is the simplest way to slot a sub-task into an in-flight feature
branch without manually resetting the auto-created branch and retargeting
the PR after the fact.

**Resolution order** (highest precedence first):

1. Explicit `Branch off:` value in the issue body — if the named branch
   exists on `origin`, it wins. If it does not exist, the handler logs a
   warning comment on the issue and falls back to step 2.
2. Sub-issue parent (`tracked-in`) branch — applies to `feature` / `task`.
3. Branch-type hierarchy default (`main` for epics/hotfixes/bugs, parent
   or `main` for features/tasks).

The handler accepts both the inline magic-line form (`Branch off: <branch>`
on its own line in the issue body) and the Issue Forms heading form (the
`### Branch off` heading followed by the value on the next line).
Placeholder values (`main`, `master`, `none`, `n/a`, `-`, `_No response_`)
are treated as "no override".

The auto-created PR is opened with `base: ${BASE_BRANCH}`, so PR
retargeting is automatic — no extra step required.

## Year Rollover

Run **Manual Version Bump** from Actions → workflow_dispatch, select `year-rollover`:
- Resets to `YY.0.0.0` when current year differs from VERSION year
- No-op if already on current year

## Release Process

1. Create PR from `main` → `release` (use **Rebase**)
2. `version-validation.yml` checks format and uniqueness vs latest tag
3. After merge, `release.yml` creates tag `v{VERSION}` and a GitHub Release

## Setting Up the Release Branch

```bash
git checkout main
git pull origin main
git checkout -b release
git push origin release
```

## Branch Protection (configure via GitHub settings)

### main
- Require PR before merging (1 approval)
- Dismiss stale reviews
- Required status checks: `validate-version`
- Require linear history
- No force pushes, no deletions

### release
- Require PR before merging (1 approval)
- Required status checks: `validate-version`
- Require linear history
- No force pushes, no deletions

### visual-explainer
- No branch protection needed — CI writes to it directly
- Do not commit to this branch manually

## Labels to Create

Create these labels in the repo for the workflows to trigger correctly:

| Label | Color | Purpose |
|-------|-------|---------|
| `epic` | `#3e4b9e` | Top-level work unit |
| `feature` | `#a2eeef` | Feature within an epic |
| `task` | `#0075ca` | Task within a feature |
| `bug` | `#d73a4a` | Bug fix |
| `hotfix` | `#b60205` | Emergency fix |
| `epic` (PR) | `#3e4b9e` | Auto-added to epic PRs |
| `addition` | `#0e8a16` | Auto-added to feature PRs |
| `implementation` | `#e4e669` | Auto-added to task PRs |
| `fix` | `#ee0701` | Auto-added to bug PRs |
| `hotfix` (PR) | `#b60205` | Auto-added to hotfix PRs |

## GitHub Pages Deployment

`deploy-docs.yml` publishes visual explainer output to GitHub Pages via the `visual-explainer` branch whenever `docs/visual-explainer/**` files change on `main`, or when triggered manually.

### How it works

1. **Build step**: checks out the repo, runs a Python script that walks `docs/visual-explainer/` subdirectories, collects `.html` files, and generates `docs/index.html` — a gallery page grouping files by type (diagrams, slides, reviews, recaps, plans).
2. **Deploy step**: mirrors the contents of `docs/` to the root of the `visual-explainer` branch using a git worktree, then pushes. The branch root becomes what GitHub Pages serves.

### Directory conventions

- `docs/visual-explainer/diagrams/` — output from `generate-web-diagram --publish`
- `docs/visual-explainer/slides/` — output from `generate-slides --publish`
- `docs/visual-explainer/reviews/` — output from `diff-review --publish` and `plan-review --publish`
- `docs/visual-explainer/recaps/` — output from `project-recap --publish`
- `docs/visual-explainer/plans/` — output from `generate-visual-plan --publish`

Only `.html` files in subdirectories are listed. Files placed directly in `docs/visual-explainer/` root are not enumerated.

### Enabling GitHub Pages

1. Go to **Settings → Pages** in the repo.
2. Set **Source** to **Deploy from a branch**.
3. Set **Branch** to `visual-explainer`, folder `/` (root).
4. Push a change to `docs/visual-explainer/**` on `main` (or run the workflow manually) to trigger the first deployment. The `visual-explainer` branch will be created automatically on first deploy.

### Permissions required

The workflow needs `contents: write` (already declared in the YAML) to push to the `visual-explainer` branch.
