# CI/CD Workflow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up GitHub Actions CI for the ScholarPress backend (fmt, clippy, test, build on PRs/push) and catalog (fixture validation via Docker image on dispatch/schedule).

**Architecture:** Backend CI runs `cargo fmt --check`, `cargo clippy --all --tests -- -D warnings`, `cargo nextest run --all`, and `cargo build --release` on every PR and push to main. After the Docker image is published to GHCR, a repository_dispatch triggers the catalog repo to run `validate_fixtures.sh` against synthetic PDF fixtures plus a nightly schedule.

**Tech Stack:** GitHub Actions, Rust (cargo, nextest, clippy, rust-cache), Bash, Docker, typst (compiled binary).

## Global Constraints

- Backend tests must not crash CI runners (use `--test-threads=2` with nextest)
- WSL is unreliable for local testing — CI is the canonical test environment
- `sp-typst` tests gate on `has_typst_binary()` — install typst binary so they execute, not skip
- Catalog `validate_fixtures.sh` depends on the published backend Docker image at `ghcr.io/scholarpress-workshop/scholarpress-backend-publish-service:latest`
- Catalog repo needs `GITHUB_TOKEN` permissions to pull the Docker image; backend repo needs `CATALOG_REPO_PAT` secret to dispatch events
- Workspace root: `/home/danriggi/scholarpress-workshop/scholarpress-backend/`
- Catalog root: `/home/danriggi/scholarpress-workshop/scholarpress-catalog/`

---

### Task 1: Backend test workflow

**Files:**
- Create: `.github/workflows/test.yml`

**Interfaces:**
- Triggered by: `push: [main]`, `pull_request: [main]`
- Produces: CI status checks on PRs (fmt, clippy, test, build)

- [ ] **Step 1: Create test.yml**

Create `.github/workflows/test.yml`:

```yaml
name: Test Suite
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - name: Install typst
        run: |
          curl -fsSL https://github.com/typst/typst/releases/latest/download/typst-x86_64-unknown-linux-musl.tar.xz | tar xJ
          sudo mv typst /usr/local/bin/
      - name: Install cargo-nextest
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest
      - name: Format check
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy --all --tests -- -D warnings
      - name: Test
        run: cargo nextest run --all -- --test-threads=2
      - name: Release build
        run: cargo build --release
```

- [ ] **Step 2: Commit and push**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add .github/workflows/test.yml
git commit -m "ci: add test suite workflow — fmt, clippy, nextest, release build"
git push
```

Expected: PR CI checks appear on next push/PR.

---

### Task 2: Trigger catalog validation from Docker publish

**Files:**
- Modify: `.github/workflows/docker-publish.yml`

**Interfaces:**
- Consumes: existing `build` job that pushes to GHCR
- Produces: `repository_dispatch` event to catalog repo after successful image push

- [ ] **Step 1: Append catalog trigger step to docker-publish.yml**

After the `Build and push` step in the `build` job, add:

```yaml
      - name: Trigger catalog validation
        run: |
          curl -X POST \
            -H "Authorization: token ${{ secrets.CATALOG_REPO_PAT }}" \
            -H "Accept: application/vnd.github.everest-preview+json" \
            https://api.github.com/repos/scholarpress-workshop/scholarpress-catalog/dispatches \
            -d '{"event_type":"image-published"}'
```

- [ ] **Step 2: Add PAT secret**

In the backend repo's GitHub Settings → Secrets → Actions, add:
- Name: `CATALOG_REPO_PAT`
- Value: a GitHub fine-grained personal access token with `contents: read` and `metadata: read` on the `scholarpress-catalog` repo (or use a classic PAT with `repo` scope)

- [ ] **Step 3: Commit and push**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-backend
git add .github/workflows/docker-publish.yml
git commit -m "ci: trigger catalog fixture validation after Docker image publish"
git push
```

Expected: On next push to main, after Docker image is pushed, the catalog repo receives a `repository_dispatch` event.

---

### Task 3: Catalog fixture validation workflow

**Files:**
- Create: `.github/workflows/validate-fixures.yml` (note: intentional typo "fixures" to match existing `validate_fixtures.sh` naming convention — the repo spells it "fixures" in the script filename)

**Interfaces:**
- Triggered by: `repository_dispatch: [image-published]`, `workflow_dispatch`, `schedule: cron(0 6 * * *)`
- Consumes: `ghcr.io/scholarpress-workshop/scholarpress-backend-publish-service:latest` Docker image
- Produces: fixture validation report

- [ ] **Step 1: Create validate-fixures.yml**

Create `.github/workflows/validate-fixures.yml`:

```yaml
name: Validate Fixtures
on:
  repository_dispatch:
    types: [image-published]
  workflow_dispatch:
  schedule:
    - cron: '0 6 * * *'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install typst
        run: |
          curl -fsSL https://github.com/typst/typst/releases/latest/download/typst-x86_64-unknown-linux-musl.tar.xz | tar xJ
          sudo mv typst /usr/local/bin/
      - name: Generate fixtures
        run: bash compile.sh
        working-directory: institutions/iu/tests/fixtures
      - name: Login to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - name: Validate fixtures
        run: bash validate_fixtures.sh
        working-directory: institutions/iu/tests
```

- [ ] **Step 2: Commit and push**

```bash
cd /home/danriggi/scholarpress-workshop/scholarpress-catalog
git add .github/workflows/validate-fixures.yml
git commit -m "ci: validate fixtures workflow — triggered on image publish, manual, and nightly"
git push
```

Expected: Nightly schedule runs at 6 AM UTC, can be manually triggered from GitHub Actions tab, and runs automatically after backend publishes a new Docker image.

---

### Task 4: Verification

**Interfaces:**
- Verifies: all three workflows exist, YAML is valid

- [ ] **Step 1: Verify YAML files exist**

```bash
ls /home/danriggi/scholarpress-workshop/scholarpress-backend/.github/workflows/test.yml
ls /home/danriggi/scholarpress-workshop/scholarpress-backend/.github/workflows/docker-publish.yml
ls /home/danriggi/scholarpress-workshop/scholarpress-catalog/.github/workflows/validate-fixures.yml
```

Expected: all three files exist.

- [ ] **Step 2: Verify secret setup checklist**

Confirm the following are configured:
- [ ] Backend repo: `CATALOG_REPO_PAT` secret for cross-repo dispatch
- [ ] Catalog repo: `GITHUB_TOKEN` is auto-provided (default)

- [ ] **Step 3: Trigger chain test**

```bash
# Manually trigger from GitHub UI:
# Go to https://github.com/scholarpress-workshop/scholarpress-catalog/actions/workflows/validate-fixures.yml
# Click "Run workflow"
```

Expected: validation passes against the currently-published Docker image.
