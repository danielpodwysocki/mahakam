# Android Emulator Viewer

## What needs to be done

Add `"android"` as a third viewer type following the same pattern as `"browser"`.

---

## Image (`images/android-viewer.containerfile`)

Base: `debian:bookworm-slim` or Ubuntu 22.04 (better SDK support).

Packages:
- Android SDK command-line tools (`cmdline-tools`)
- Android emulator package (`emulator`)
- A specific system image — recommend `system-images;android-34;google_apis;x86_64`
- Xvfb, x11vnc, noVNC / websockify (same as browser viewer)
- OpenJDK 17 (required by SDK tools)

AVD creation at build time (bake a pre-created AVD into the image):
```sh
sdkmanager "platform-tools" "emulator" "platforms;android-34" \
    "system-images;android-34;google_apis;x86_64"
avdmanager create avd -n mahakam -k "system-images;android-34;google_apis;x86_64" \
    --device pixel_4 --force
```

Start script:
```sh
Xvfb :99 -screen 0 1280x800x24 -ac &
emulator -avd mahakam -no-audio -gpu swiftshader_indirect -no-boot-anim &
x11vnc -display :99 -forever -nopw -rfbport 5900 -shared -quiet &
websockify --web=/usr/share/novnc/ 6080 localhost:5900
```

Expected image size: 6–8 GB. Pin the SDK and system image versions explicitly.

---

## Kubernetes requirements

The emulator requires `/dev/kvm` (hardware-accelerated virtualisation).
Without it `swiftshader_indirect` (software) falls back but is very slow.

Pod spec additions:
```yaml
securityContext:
  privileged: true          # or use the device plugin approach below
volumes:
  - name: kvm
    hostPath:
      path: /dev/kvm
volumeMounts:
  - name: kvm
    mountPath: /dev/kvm
```

Kind clusters on Linux with a KVM-capable host work. Kind on macOS/Windows does not
expose `/dev/kvm` to nodes, so the emulator cannot run there.

---

## API changes (`environments.rs`)

Add to `viewer_spec_for`:
```rust
"android" => Some(crate::k8s::viewer::ViewerSpec {
    name: "android".to_string(),
    display_name: "Android Emulator".to_string(),
    image: android_viewer_image.to_string(),
    path_prefix: format!("/projects/viewers/{env_name}/android"),
    port: 6080,
    env_vars: vec![],
    strip_path_prefix: true,
}),
```

Add `android_viewer_image: Arc<String>` to `AppState` and a corresponding
`ANDROID_VIEWER_IMAGE` env var / `--android-viewer-image` CLI flag.

The Deployment spec needs the `/dev/kvm` volume and mount. `ViewerSpec` may need
a `volumes` / `volume_mounts` field, or a separate `AndroidViewerSpec` that
`spawn_viewer` handles as a special case.

---

## Frontend changes

Add `{ name: 'android', label: 'Android Emulator' }` to `AVAILABLE_VIEWERS`
in `EnvironmentForm.vue`. No other frontend changes needed — the viewer button
appears automatically from the HTTPRoute labels.

---

## Notes

- Cold start: AVD boot takes 1–3 minutes even with a pre-baked image. The
  provisioning background task will finish before the emulator is ready, so the
  viewer button becomes clickable but the screen may show a black frame for a while.
- Consider a readiness probe on the pod that checks `adb shell getprop sys.boot_completed`.
- `swiftshader_indirect` rendering is CPU-bound and slow for GPU-heavy apps.
  Real GPU passthrough is not feasible in Kind; use a bare-metal or VM-backed cluster.
