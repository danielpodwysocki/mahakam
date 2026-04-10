# syntax=docker/dockerfile:1
# Android viewer: Android Emulator + Xvfb + x11vnc + noVNC behind nginx.
# Requires /dev/kvm mounted in the pod for hardware-accelerated virtualisation.
# Without it the emulator falls back to swiftshader_indirect (software rendering, slow).
#
# WS_NAME is injected at runtime. The entrypoint:
#   1. Generates /usr/share/novnc/index.html — a tiny JS redirect to vnc.html with
#      the correct ?path= so noVNC knows its WebSocket endpoint.
#   2. Templates the nginx config with the path prefix and starts nginx.
#
# nginx handles everything at the correct prefix — no gateway-level rewrites.
# websockify runs as a pure WS→TCP proxy; nginx owns all HTTP serving.
FROM debian:bookworm-slim

ENV ANDROID_HOME=/opt/android-sdk
ENV ANDROID_SDK_ROOT=/opt/android-sdk
ENV PATH="${ANDROID_HOME}/cmdline-tools/latest/bin:${ANDROID_HOME}/platform-tools:${ANDROID_HOME}/emulator:${PATH}"

# Pinned: cmdline-tools 11.0 (build 10406996)
ARG CMDLINE_TOOLS_ZIP=commandlinetools-linux-10406996_latest.zip

RUN apt-get update && apt-get install -y --no-install-recommends \
        openjdk-17-jdk-headless \
        wget \
        unzip \
        xvfb \
        x11vnc \
        novnc \
        websockify \
        nginx \
        gettext-base \
        libpulse0 \
        libglib2.0-0 \
        libx11-6 \
        libc6 \
        libstdc++6 \
    && rm -rf /var/lib/apt/lists/* \
    && rm -f /etc/nginx/sites-enabled/default

# Install Android cmdline-tools into ${ANDROID_HOME}/cmdline-tools/latest
RUN mkdir -p ${ANDROID_HOME}/cmdline-tools && \
    wget -q "https://dl.google.com/android/repository/${CMDLINE_TOOLS_ZIP}" \
        -O /tmp/cmdline-tools.zip && \
    unzip -q /tmp/cmdline-tools.zip -d /tmp/ct && \
    mv /tmp/ct/cmdline-tools ${ANDROID_HOME}/cmdline-tools/latest && \
    rm -rf /tmp/cmdline-tools.zip /tmp/ct

# Accept licenses and install the exact SDK packages needed
RUN yes | sdkmanager --licenses && \
    sdkmanager \
        "platform-tools" \
        "emulator" \
        "platforms;android-34" \
        "system-images;android-34;google_apis;x86_64"

# Bake a pre-created AVD into the image so the emulator boots without extra setup.
RUN avdmanager create avd \
        -n mahakam \
        -k "system-images;android-34;google_apis;x86_64" \
        --device pixel_4 \
        --force

# nginx config template.
# Only ${WS_NAME} is substituted at startup; $http_upgrade is left as-is.
COPY <<'NGINX_TMPL' /etc/nginx/novnc.conf.template
server {
    listen 6080;

    # WebSocket proxy — must be matched before the general static block.
    location ^~ /projects/viewers/${WS_NAME}/android/websockify {
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
    # served correctly.
    location ^~ /projects/viewers/${WS_NAME}/android/ {
        alias /usr/share/novnc/;
        index index.html;
    }
}
NGINX_TMPL

COPY <<'ENTRY' /entrypoint.sh
#!/bin/sh
set -e

PREFIX="projects/viewers/${WS_NAME}/android"

# Generate a thin index page that redirects the browser to vnc.html with the
# correct WebSocket path.
cat > /usr/share/novnc/index.html <<HTML
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Android Viewer — ${WS_NAME}</title>
<script>
window.location = '/${PREFIX}/vnc.html?path=${PREFIX}/websockify&autoconnect=1';
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

ANDROID_EMULATOR_HOME=/root/.android \
DISPLAY=:99 emulator \
    -avd mahakam \
    -no-audio \
    -gpu swiftshader_indirect \
    -no-boot-anim \
    -no-snapshot \
    -memory "${EMULATOR_MEMORY_MB:-2048}" &

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
ENV SCREEN_RESOLUTION=1280x800x24
ENV EMULATOR_MEMORY_MB=2048

EXPOSE 6080

CMD ["/entrypoint.sh"]
