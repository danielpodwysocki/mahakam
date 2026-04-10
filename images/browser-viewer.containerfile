# syntax=docker/dockerfile:1
# Browser viewer: Chromium + Xvfb + x11vnc + noVNC behind nginx.
#
# WS_NAME is injected at runtime. The entrypoint:
#   1. Generates /usr/share/novnc/index.html — a tiny JS page that redirects
#      the browser to vnc.html with the correct ?path= so noVNC knows its
#      WebSocket endpoint. curl / e2e tests see HTTP 200 from nginx directly.
#   2. Templates the nginx config with the path prefix and starts nginx.
#
# nginx handles everything at the correct prefix — no gateway-level rewrites.
# websockify runs as a pure WS→TCP proxy; nginx owns all HTTP serving.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        chromium \
        xvfb \
        x11vnc \
        novnc \
        websockify \
        nginx \
        gettext-base \
    && rm -rf /var/lib/apt/lists/* \
    && rm -f /etc/nginx/sites-enabled/default

# nginx config template.
# Only ${WS_NAME} is substituted at startup; $http_upgrade is left as-is.
COPY <<'NGINX_TMPL' /etc/nginx/novnc.conf.template
server {
    listen 6080;

    # WebSocket proxy — must be matched before the general static block.
    location ^~ /projects/viewers/${WS_NAME}/browser/websockify {
        proxy_pass         http://127.0.0.1:5901/;
        proxy_http_version 1.1;
        proxy_set_header   Upgrade    $http_upgrade;
        proxy_set_header   Connection "upgrade";
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
    }

    # Static noVNC files: alias strips the location prefix so nginx maps
    # /prefix/X → /usr/share/novnc/X without any internal URI rewriting.
    # The index subrequest stays within this location, so index.html is
    # served correctly. The generated index.html (written at startup) tells
    # noVNC where its WebSocket path lives.
    location ^~ /projects/viewers/${WS_NAME}/browser/ {
        alias /usr/share/novnc/;
        index index.html;
    }
}
NGINX_TMPL

COPY <<'ENTRY' /entrypoint.sh
#!/bin/sh
set -e

PREFIX="projects/viewers/${WS_NAME}/browser"

# Generate a thin index page that redirects the browser to vnc.html with the
# correct WebSocket path. This avoids HTTP redirect loops and lets curl / e2e
# tests get a clean 200 from /prefix/ without any chained redirects.
cat > /usr/share/novnc/index.html <<HTML
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Browser Viewer — ${WS_NAME}</title>
<script>
window.location = '/${PREFIX}/vnc.html?path=${PREFIX}/websockify&autoconnect=1&resize=scale';
</script>
</head>
<body>Loading viewer&hellip;</body>
</html>
HTML

# Template the nginx config with the current WS_NAME.
envsubst '${WS_NAME}' \
    < /etc/nginx/novnc.conf.template \
    > /etc/nginx/conf.d/novnc.conf

nginx -t

Xvfb :99 -screen 0 "${SCREEN_RESOLUTION:-1280x800x24}" -ac +extension GLX -noreset &
sleep 1

DISPLAY=:99 chromium \
    --no-sandbox \
    --disable-dev-shm-usage \
    --disable-gpu \
    "${BROWSER_URL:-about:blank}" &

x11vnc \
    -display :99 \
    -forever \
    -nopw \
    -rfbport 5900 \
    -shared \
    -quiet &

websockify 5901 localhost:5900 &

sleep 1
exec nginx -g "daemon off;"
ENTRY

RUN chmod +x /entrypoint.sh

ENV WS_NAME=default
ENV BROWSER_URL=about:blank
ENV SCREEN_RESOLUTION=1280x800x24

EXPOSE 6080

CMD ["/entrypoint.sh"]
