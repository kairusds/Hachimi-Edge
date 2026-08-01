# Building Hachimi Edge (Linux / Arch Linux)

This guide provides instructions for cross-compiling **Hachimi Edge** for Android (`aarch64-linux-android`) on Linux, specifically tailored for Arch Linux environments (such as Waydroid setups).

---

## 1. Prerequisites

Install the required build dependencies on Arch Linux:

```bash
sudo pacman -S --noconfirm rustup android-ndk cargo-ndk
```

Set up Rust and add the Android target:

```bash
rustup default stable
rustup target add aarch64-linux-android
```

---

## 2. Environment & Linker Configuration

### Environment Variables

Export the Android NDK path:

```bash
export ANDROID_NDK_HOME=/opt/android-ndk
export PATH=/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH
```

### Cargo Configuration

Ensure `.cargo/config.toml` exists in the repository root with the following content to link against the NDK toolchain and static C++ runtime (`libc++_static.a`):

```toml
[target.aarch64-linux-android]
linker = "/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
ar = "/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
rustflags = [
    "-C", "link-arg=/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_static.a",
    "-C", "link-arg=/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++abi.a"
]
```

---

## 3. Building

To compile the `libhachimi.so` shared library in `release` mode:

```bash
cargo build --target aarch64-linux-android --release
```

The compiled shared object will be located at:
`target/aarch64-linux-android/release/libhachimi.so`

---

## 4. Installing into Waydroid / Android

1. Push the compiled `.so` file to `/data/local/tmp/libmain.so`:

   ```bash
   adb push target/aarch64-linux-android/release/libhachimi.so /data/local/tmp/libmain.so
   ```

2. Replace `libmain.so` inside the target application package directory using `waydroid shell`:

   ```bash
   sudo waydroid shell -- sh -c '
   cp /data/local/tmp/libmain.so /data/app/~~*/jp.co.cygames.umamusume-*/lib/arm64/libmain.so
   chmod 755 /data/app/~~*/jp.co.cygames.umamusume-*/lib/arm64/libmain.so
   '
   ```

3. Restart the game:

   ```bash
   adb shell "am force-stop jp.co.cygames.umamusume"
   adb shell "am start -n jp.co.cygames.umamusume/jp.co.cygames.umamusume_activity.UmamusumeActivity"
   ```

---

## Headless / Auto-Update Mode Notes

If running in headless mode (e.g. `disable_gui = true`), Hachimi automatically skips first-time setup dialogs and fetches translation repositories (`https://raw.githubusercontent.com/UmaTL/hachimi-tl-en/release/index.json`) in the background on startup without requiring any GUI overlay interaction.
