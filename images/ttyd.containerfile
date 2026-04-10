# syntax=docker/dockerfile:1
# Upstream ttyd Alpine build — published by tsl0922/ttyd CI to ghcr.io/tsl0922/ttyd:alpine.
# ENV_NAME is injected at runtime by the Kubernetes Deployment so each instance
# is scoped to its own path prefix: /projects/viewers/<ENV_NAME>.
FROM ghcr.io/tsl0922/ttyd:alpine

# Shell-form CMD: /bin/sh -c expands ${ENV_NAME} before launching ttyd.
CMD ttyd --base-path "/projects/viewers/${ENV_NAME}/terminal" --writable sh
