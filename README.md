# Mahakam

Workspace provisioning platform — ArgoCD + vcluster + Envoy Gateway, Nuxt UI.

## Quick start

```sh
task install              # bootstrap Kind cluster + full stack (~5 min)
task port-forward-gateway # forward Envoy Gateway → localhost:8080
```

## Dev loop

```sh
task develop   # Skaffold hot-reload for API + web UI
task test      # Rust + frontend unit tests
task lint      # clippy + fmt + vue-tsc
```

## Viewer images

Viewer images (`mahakam-ttyd`, `mahakam-browser-viewer`, `mahakam-android-viewer`) are **not** managed by Skaffold. When you rebuild one, load it manually:

```sh
docker build -f images/browser-viewer.containerfile -t mahakam-browser-viewer:latest .
kind load docker-image mahakam-browser-viewer:latest --name <cluster-name>
```
