## Copilot instructions

This file contains instructions for how to work in this codebase. Follow these guidelines strictly.


## Project structure - design and layout

The project structure follows Workspace-Oriented Design.
The philosophy is "what deploys together, lives together in a single git repository".

That means you can have multiple services here — for example a batch processing crate and an API crate as separate workspace members.
Attempt to keep the amount of API services minimal. When you add one, update the project-specific structure section below in this file.

For data/business logic separation, we use the service/repository pattern: all business logic lives in services, all data handling in repositories.

The workspace layout looks as follows:

- `Cargo.toml` - workspace manifest listing all member crates
- `crates/` - one directory per deployable or shared crate
  - `crates/shared/` - **mandatory shared library**: repository traits, models, and services reusable across multiple programs
    - `src/lib.rs` - re-exports all public modules
    - `src/services/` - business logic reusable across programs
    - `src/repositories/` - repository traits, models, and implementations
  - `crates/api/` - HTTP API service (depends on `crates/shared`)
    - `src/main.rs` - entry point
    - `src/routes/` - axum router and handler registration
    - `src/config.rs` - configuration loaded from env/flags
    - `src/error.rs` - application-level error type
  - `crates/<cli-name>/` - CLI programs (depend on `crates/shared`)
    - `src/main.rs` - entry point, clap setup
- `tests/` - integration tests (one file per program under test)

**The rule for placement — no exceptions:**
- Services and repositories **always** go in `crates/shared/`, regardless of whether they are currently used by more than one program
- Individual program crates contain **only** transport-layer code: HTTP handlers, axum routers, CLI argument structs, `Config`, `main.rs`, and program-specific error types
- If you are unsure whether something is "transport-layer", ask: does it touch the network, CLI args, or program lifecycle? If no → it belongs in `crates/shared/`

Under `crates/` you can put programs of all kinds: CLI tools, daemons, web services.
For web services, assuming service `hello` as an example, follow this structure:
- `crates/hello/src/main.rs` - entry point, server setup
- `crates/hello/src/routes/` - axum Router construction and handlers
- `crates/hello/src/routes/handlers/` - individual handler functions
- `crates/hello/tests/` - integration tests

Repositories must only ever be imported in services, never directly in handlers.

Each `crates/shared/src/repositories/<RepositoryName>/` follows this structure:
- `mod.rs` - the trait definition that all implementations must satisfy
- `models.rs` - data models returned by the repository
- `mock.rs` - mock implementation with no external connectors (for unit tests)
- `<datasource>.rs` - specific implementations (e.g., `postgres.rs`, `mysql.rs`)


Only add new programs when explicitly requested in the task specification.
When adding a new deployable program (web service, CLI, daemon), you must:
1. Create `crates/<name>/` with `Cargo.toml` (declare `shared = { path = "../shared" }`) and `src/main.rs`
2. Add the crate to the workspace `Cargo.toml` members list
3. Place any new business logic or repository code in `crates/shared/`, not in the new crate
4. For web services: create `images/<name>.containerfile`, update `skaffold.yaml`, create `kube/<name>-deployment.yaml`, update `kube/kustomization.yaml`
5. Update the "Project structure - project specific" section in this file



## Project structure - project specific

This section documents project-specific services, repositories, and their responsibilities.
**IMPORTANT:** Update this section whenever you add a new service or repository.

### Services

#### api (crates/api)
HTTP API service providing REST endpoints for workspace and project CRUD. Provisions vclusters
via ArgoCD Applications, applies kustomize overlays via kubectl, and spawns viewer pods with
HTTPRoute rules via kube-rs.

### Repositories

#### WorkspaceRepository (crates/shared/src/repositories/workspace/)
**NOT used by the API handlers.** The API uses ArgoCD Applications as its workspace database
(see "ArgoCD as database" below). This repository and its SQLite implementation exist in
`crates/shared` for completeness and future use, but `crates/api` does not import them.
- `mod.rs` — trait + `Workspace` struct (fields: id, name, repos, namespace, status, created_at, viewers, project)
- `sqlite.rs` — SQLite implementation (unused by API)
- `mock.rs` — mockall mock used in `WorkspaceService` unit tests

### Shared Crate (crates/shared)

#### Services

##### WorkspaceService (crates/shared/src/services/workspace.rs)
Validates workspace names (lowercase alphanum + hyphens, 1–63 chars). 100% unit test coverage
enforced. Does NOT call K8s — that happens in API handlers directly.

---

## Runtime architecture

### ArgoCD as database

Workspace state lives entirely in ArgoCD `Application` objects in the ArgoCD namespace, not in
SQLite. The API reads and writes workspace metadata via annotations on these objects. **This means
the API is stateless** — pod restarts lose nothing.

Each workspace has one outer ArgoCD Application named `ws-{name}` with these annotations:

| Annotation | Content |
|---|---|
| `mahakam.io/ws-id` | UUID string |
| `mahakam.io/ws-repos` | JSON array of repo URLs |
| `mahakam.io/ws-created-at` | RFC 3339 timestamp |
| `mahakam.io/ws-status` | `"pending"` \| `"failed"` (ArgoCD health drives `"ready"`) |
| `mahakam.io/ws-project` | Project name string, default `"default"` |

Label `mahakam.io/managed=true` on the Application lets the API list only mahakam-managed workspaces.

The outer Application sources `chart/workspace/` from the mahakam git repo. When ArgoCD syncs it,
two child resources are created (sync waves):
- wave -1: `Namespace ws-{name}` (labeled `mahakam.io/managed=true`)
- wave 0: inner `Application vcluster-{name}` (installs the vcluster Helm chart)

Cascade deletion via `resources-finalizer.argocd.argoproj.io` unwinds everything when the outer
Application is deleted.

### Projects

Projects are **not** a Kubernetes resource. They are derived at query time from the
`mahakam.io/ws-project` annotation across all workspace Applications. Creating a workspace with
`project: "my-project"` implicitly creates that project. `GET /api/v1/projects` always includes
`"default"` even when no workspaces reference it.

### REST API surface

```
GET    /api/v1/workspaces                    list all workspaces (viewers merged from HTTPRoutes)
POST   /api/v1/workspaces                    create workspace + background provisioning
GET    /api/v1/workspaces/{name}             single workspace with viewers
DELETE /api/v1/workspaces/{name}             teardown viewers + delete ArgoCD Application
GET    /api/v1/workspaces/{name}/metrics     pod count + CPU/memory from ws-{name} namespace
GET    /api/v1/projects                      project list derived from annotations
GET    /api/v1/projects/{name}/workspaces    workspaces filtered by project
GET    /api/v1/health                        health check
```

### Viewer architecture

After provisioning, the API spawns "viewers" (terminal, browser, android) as separate pods in
`ws-{name}`. Each viewer creates:
- `Deployment` + `Service` in `ws-{name}`
- `ReferenceGrant` in `ws-{name}` (allows cross-namespace backendRef)
- `HTTPRoute` in `mahakam-system` with labels `mahakam.io/viewer=true` and `mahakam.io/ws-name={ws}`

The HTTPRoute labels are how the API discovers active viewers at list time (`list_all_ws_viewers`).
Display name and path are stored in HTTPRoute annotations (`mahakam.io/viewer-display-name`,
`mahakam.io/viewer-path`). Viewer routes are at `/projects/viewers/{ws}/{viewer-name}/`.

`ViewerSpec` in `crates/api/src/k8s/viewer.rs` controls: image, port, env vars, path prefix,
`strip_path_prefix` (URLRewrite filter on the HTTPRoute), `privileged`, and `host_devices`
(hostPath volumes — needed for `/dev/kvm` in the Android emulator viewer).

### crates/api/src/k8s/ modules

| Module | Responsibility |
|---|---|
| `argocd.rs` | Create/list/delete/status-patch workspace Applications |
| `vcluster.rs` | Wait for vcluster kubeconfig secret; probe vcluster API |
| `kustomize.rs` | Render and apply per-workspace kustomize overlay |
| `viewer.rs` | Spawn/teardown viewer Deployments + HTTPRoutes; list active viewers |
| `metrics.rs` | Query pod resource requests/limits in `ws-{name}` namespace |

### workspaces/base/
Kustomization base template. Mahakam renders a per-workspace overlay at runtime pointing at
this base. Do **not** add workspace-specific content here; the API generates overlays in temp dirs.

### chart/mahakam/
Production Helm chart that bundles the mahakam API, web UI, and ArgoCD.
Use `task install` to bootstrap the full stack into a Kind cluster.

### kube/
Dev manifests (no Helm). Used by Skaffold for hot-reload development (`task develop`).

### frontends/web/
Nuxt 3 web UI. Unit tests run with vitest; 100% coverage enforced on `api/`, `composables/`,
`components/`, `stores/`.

**Page routing:**
- `/` — project card grid (`pages/index.vue`)
- `/projects/[project]` — workspace list + create form for a project (`pages/projects/[project].vue`)
- `/projects/[project]/workspaces/[name]` — workspace detail with left sidebar nav and main area
  showing either a stats dashboard (pod count, CPU, memory) or a full-height viewer iframe
  (`pages/projects/[project]/workspaces/[name].vue`)

**Server routes** in `frontends/web/server/api/` proxy all API calls server-side so the backend
address is never exposed to the browser. The `apiBaseUrl` runtime config key controls the backend
URL (server-only, never leaked to client).

**Vitest setup:** `tests/unit/setup.ts` stubs `NuxtLink` globally as a plain `<a>` so components
using `<NuxtLink>` can be tested without a full Nuxt context.

### Namespace labels
Every workspace namespace is labeled `mahakam.io/managed=true`. Never delete labeled namespaces
manually — use the DELETE `/api/v1/workspaces/:name` endpoint.

### Running the stack
- `task install` — bootstrap Kind + vCluster + Envoy Gateway + mahakam via Helm (takes a few minutes)
- `task develop` — Skaffold hot-reload dev loop (requires Kind cluster with EG installed)
- `task port-forward-gateway` — forward the Envoy Gateway to localhost:8080 (web + API together)
- `task test` — Rust unit tests + frontend vitest
- `task coverage` — 100% line coverage gate on shared services

### Envoy Gateway
Traffic entry point for the mahakam stack. Lives in `envoy-gateway-system` (Helm) with the routing
config in `kube/gateway.yaml`.
- `GatewayClass` `mahakam` → Envoy Gateway controller
- `Gateway` `mahakam` in `mahakam-system`, port 80
- `HTTPRoute` `mahakam-api`: `/api/v1/*` → `mahakam-api:3000`
- `HTTPRoute` `mahakam-web`: `/*` → `mahakam-web:3001`

After `task install`, run `task port-forward-gateway` and open http://localhost:8080.
- `task coverage-frontend` — 100% coverage gate on frontend layers


## Tools available

The `task` command (a make alternative) is installed and available.
Available task targets defined in `Taskfile.yml`:

- `task dev` - Start development environment with Skaffold
- `task build` - Build all binaries in release mode
- `task run` - Run the API locally
- `task test` - Run all tests with cargo-nextest
- `task coverage` - Measure service coverage; fails if < 100% line coverage on crates/shared/src/services
- `task lint` - Run clippy and check formatting
- `task fmt` - Auto-format all code
- `task cluster` - Create Kind cluster
- `task cleanup` - Delete Kind cluster
- `task docker-build` - Build container image locally
- `task project-init` - Initialize project by replacing rust-template with current directory name


The environment is managed by Nix flakes (defined in `flake.nix` at repo root).
To enter the development environment: `nix develop` or use direnv with `.envrc`

Available CLI tools in the Nix environment:
- rustup (manages Rust toolchain — run `rustup toolchain install stable` on first use)
- rust-analyzer (Rust language server)
- cargo-nextest (faster test runner)
- cargo-watch (file watcher for auto-rebuild)
- go-task (task runner, a make alternative)
- skaffold (Kubernetes deployment tool)
- kubectl (Kubernetes CLI)
- kind (local Kubernetes clusters)
- docker (container runtime)
- git
- k9s (Kubernetes TUI)

You also have access to standard shell utilities.


## How to validate changes

After writing a function that exposes behavior in a service:
1. Run `task lint` and fix issues until it passes
2. Write unit tests for the function
3. Run `task test` and fix until tests pass

When making changes to services:
1. Use `task run` to test locally
2. Use `task dev` to test in Kubernetes with hot reload
3. Verify the service responds correctly


## How to write code

### Code review markers
- Mark uncertain code with `// CIFail: Human Review Required` followed by details about security implications, code style, performance concerns, or data expectations

### Rust style and structure
- Use standard Rust naming conventions: `snake_case` for functions, variables, modules, and fields; `PascalCase` for types, structs, enums, and traits; `SCREAMING_SNAKE_CASE` for constants
- Write single-responsibility functions that accomplish one meaningful task — avoid both micro-functions and bloated ones
- Abstract only when it improves clarity
- Data models describe a single concept
- Abstract late rather than early — only when the abstraction is complete, predictable, and leaks no implementation details
- Prefer newtypes over raw primitives for domain concepts (e.g., `struct UserId(i64)` over bare `i64`)

### Comments
- Comment public items for rustdoc generation (succinct style using `///`)
- Only write non-rustdoc comments when describing obscure upstream behavior or non-obvious invariants

### Kubernetes / YAML construction

**Never build YAML from strings** (no `format!`, `concat!`, raw strings with substitutions, or `\n\` line-continuation). Rust's line-continuation escape eats leading whitespace on the next source line, silently corrupting YAML indentation in ways that are invisible to the compiler and hard to spot in review.

**Always use `serde_json::json!()` + `serde_yaml::to_string()`** to construct any YAML document — Kubernetes manifests, Helm values, kustomize overlays, patch bodies, etc.:

```rust
// ✅ correct
let yaml = serde_yaml::to_string(&serde_json::json!({
    "apiVersion": "v1",
    "kind": "ConfigMap",
    "metadata": { "name": name, "namespace": ns },
    "data": { "key": value },
}))?;

// ❌ wrong — \n\ strips indentation; concat!/raw strings are fragile
let yaml = format!("apiVersion: v1\nkind: ConfigMap\n  name: {name}\n");
```

This applies equally to nested YAML (e.g. a kustomize `patch:` field whose value is itself a YAML document — serialize the inner doc with `serde_yaml::to_string()` first, then embed the resulting string in the outer `json!`).

### Architecture patterns
- All data access must go through the repository pattern — no direct data operations outside repositories
- Define each repository as a trait; pass implementations via dependency injection (function parameters or struct fields)
- This ensures pure business logic in the service layer
- Mock all repositories in unit tests by passing mock implementations (use `mockall` crate)
- Core code MUST NEVER contain test-specific functionality or conditionals — only mock repositories should be test-aware

### Testing

**Coverage requirements:**
- `crates/shared/src/services/` — **100% line coverage enforced**. `task coverage` fails the build if any service line is uncovered. There are no exceptions; write the test before closing the task.
- `crates/shared/src/repositories/` — no coverage requirement. Repository implementations are integration-tested against a real data source, not unit-tested.
- Individual program crates — no coverage requirement. Handlers and CLI glue are validated via e2e tests.

**Rules:**
- Write unit tests as you develop; run `task test` continuously
- Place unit tests in the same file as the code under test, inside a `#[cfg(test)]` module
- Place integration tests in `tests/` at the crate root
- Cover unexpected usage and bad data in service tests
- Mock all repositories in service unit tests via trait injection (`mockall`)

### Data models
- Reuse data models from stable upstream APIs only when they correctly represent the data being handled
- Derive `serde::Serialize` / `serde::Deserialize` on models that cross serialization boundaries

### Focus and context
- Work on one crate/module at a time
- Avoid reading files outside the current focus unless necessary

### Error handling
- Use `thiserror` to define typed error enums in library/service code
- Use `anyhow` for application-level error propagation in binaries
- Use `?` for propagation; avoid `.unwrap()` except in tests
- Use `panic!` only for truly impossible program states (invariant violations)
- Be explicit — avoid silent fallback logic when not warranted
- Mark code with retry logic with `// CIFail: Human Review Required`


## Rust-specific details

### Logging
- Use `tracing` with `tracing-subscriber` for structured logging
- Instrument async functions with `#[tracing::instrument]`

### Configuration
- Use `clap` with the `derive` feature for CLI arguments
- Use `clap`'s `env` attribute to fall back to environment variables with corresponding names
- Example: `#[arg(long, env = "API_PORT", default_value = "3000")]`

### Dependencies
- Prefer the standard library where convenient
- Use `sqlx` for raw SQL queries — the repository layer is your ORM
- Avoid ORMs that obscure the SQL being executed

### HTTP framework
- Use `axum` as the HTTP framework
- Structure routers using `axum::Router::new()` and nest sub-routers per service
- Return `impl IntoResponse` from handlers; use typed extractors (`Json<T>`, `Path<T>`, etc.)

### Performance awareness
- Mark performance-sensitive code with `// CIWarning: Perf Review Recommended`
- Apply this to:
  - Allocations in hot paths
  - Large serialization operations
  - Data-intensive queries handling large datasets


## How to design APIs

When creating web endpoints:

### Endpoint structure
- Always version endpoints: `/<program name>/v1/<service name>`
  - `<service name>` corresponds to services in `src/services/`
  - `<program name>` is the crate name in `crates/`

### Updates
- Use field masks (partial update structs) when performing updates on data


## Writing tests as you develop

A test container with mounted test files is available in the local development cluster.

Start it with: `task cluster-up`

This builds and brings up:
- The main service container and all e2e test dependencies
- The test container for running tests

You can exec into the dev/test container via: `kubectl exec -n default -it deployments/dev-tooling -- <command>`


## Validating your changes

Run `task lint` immediately after every Rust code change — before running tests, before
deploying, before asking for review. Clippy catches wrong API usage, missing trait imports,
and type mismatches that would otherwise only surface as compile errors or runtime failures
inside a container. Fix all warnings before proceeding.

Full validation sequence:
- after every code edit: `task lint` (clippy + fmt check + frontend typecheck)
- once the change is ready: `task test`
- then ensure service coverage is 100%: `task coverage` (fails if any service line is uncovered)
- then ensure your code passes the e2e suite: `task verify-all`


## How to navigate this codebase

### Dependencies
- Check `Cargo.toml` for direct dependencies; `Cargo.lock` for pinned transitive versions
- Only look up crate source when investigating unclear upstream behavior or searching for reusable types
- Exclude `target/` from all searches


## Dependency management

- Always commit `Cargo.lock` for binaries — it is the reproducible build record
- Use workspace-level `[dependencies]` in the root `Cargo.toml` to share dependency versions across crates
- Pin versions explicitly for security-sensitive dependencies
