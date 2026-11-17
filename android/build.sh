#!/usr/bin/env bash
#
# Build a FRGLib game for Android.
#
#   ./android/build.sh                 # build the game named in game.properties
#   ./android/build.sh --install       # ... and install it on the connected device
#   ./android/build.sh --template      # build the bundled smoke-test crate instead
#   ./android/build.sh --debug         # cargo profile debug (the APK is still debug-signed)
#   ./android/build.sh --abi x86_64    # override the ABI list for one build
#
# The script does four things, in order:
#   1. locates the Android SDK/NDK and writes android/local.properties;
#   2. runs `cargo ndk` for the requested ABIs;
#   3. copies libmain.so, libSDL3.so and libSDL3_ttf.so into jniLibs/<abi>/;
#   4. copies assets and invokes Gradle.
#
# Environment overrides: ANDROID_HOME, ANDROID_NDK_HOME, GRADLE_TASK.

set -euo pipefail

ANDROID_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$ANDROID_DIR/.." && pwd)"
GAME_PROPERTIES="$ANDROID_DIR/game.properties"

TEMPLATE=0
INSTALL=0
PROFILE="release"
ABI_OVERRIDE=""
GRADLE_TASK_OVERRIDE="${GRADLE_TASK:-}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --template) TEMPLATE=1 ;;
        --install) INSTALL=1 ;;
        --debug) PROFILE="debug" ;;
        --release) PROFILE="release" ;;
        --abi)
            shift
            [[ $# -gt 0 ]] || { echo "--abi needs a value" >&2; exit 2; }
            ABI_OVERRIDE="$1"
            ;;
        -h|--help)
            sed -n '2,17p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            exit 2
            ;;
    esac
    shift
done

if [[ ! -f "$GAME_PROPERTIES" ]]; then
    echo "missing $GAME_PROPERTIES" >&2
    exit 1
fi

# Reads `key=value` from game.properties. Values are plain text: no quoting and
# no escapes, which keeps the parser a one-liner.
prop() {
    sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*//p" "$GAME_PROPERTIES" \
        | head -n1 \
        | tr -d '\r'
}

CRATE="$(prop crate)"
ASSETS="$(prop assets)"
PACKAGE="$(prop package)"
APP_NAME="$(prop app_name)"
ORIENTATION="$(prop orientation)"
ABIS="$(prop abis)"
MIN_SDK="$(prop min_sdk)"
NDK_VERSION="$(prop ndk_version)"

if [[ "$TEMPLATE" -eq 1 ]]; then
    CRATE="android/template"
    ASSETS=""
    PACKAGE="org.frglib.template"
    APP_NAME="FRGLib Template"
    ORIENTATION="landscape"
fi

: "${CRATE:?crate is missing in game.properties}"
: "${PACKAGE:?package is missing in game.properties}"
: "${ABIS:=arm64-v8a,x86_64}"
: "${MIN_SDK:=24}"

if [[ -n "$ABI_OVERRIDE" ]]; then
    ABIS="$ABI_OVERRIDE"
fi

# --- SDK and NDK ---------------------------------------------------------

find_sdk() {
    local candidate
    for candidate in \
        "${ANDROID_HOME:-}" \
        "${ANDROID_SDK_ROOT:-}" \
        "$HOME/Documents/Android/android-sdk" \
        "$HOME/Documents/Android/Sdk" \
        "$HOME/Android/Sdk" \
        "$HOME/Library/Android/sdk"; do
        if [[ -n "$candidate" && -d "$candidate/platform-tools" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

find_ndk() {
    local sdk="$1"
    local requested="${2:-}"

    if [[ -n "${ANDROID_NDK_HOME:-}" && -d "$ANDROID_NDK_HOME" ]]; then
        printf '%s\n' "$ANDROID_NDK_HOME"
        return 0
    fi
    if [[ -n "${ANDROID_NDK_ROOT:-}" && -d "$ANDROID_NDK_ROOT" ]]; then
        printf '%s\n' "$ANDROID_NDK_ROOT"
        return 0
    fi
    if [[ -n "$requested" && "$requested" != "auto" && -d "$sdk/ndk/$requested" ]]; then
        printf '%s\n' "$sdk/ndk/$requested"
        return 0
    fi
    if [[ -d "$sdk/ndk" ]]; then
        local newest
        newest="$(find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null \
            | sort -V | tail -n1)"
        if [[ -n "$newest" ]]; then
            printf '%s\n' "$sdk/ndk/$newest"
            return 0
        fi
    fi
    return 1
}

if ! SDK="$(find_sdk)"; then
    echo "Android SDK not found. Set ANDROID_HOME to a directory containing platform-tools/." >&2
    exit 1
fi

if ! NDK="$(find_ndk "$SDK" "${NDK_VERSION:-auto}")"; then
    echo "Android NDK not found under $SDK/ndk. Set ANDROID_NDK_HOME." >&2
    exit 1
fi
NDK_VERSION="$(basename "$NDK")"

export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"
export ANDROID_NDK_HOME="$NDK"
export ANDROID_NDK_ROOT="$NDK"
export CMAKE_TOOLCHAIN_FILE="$NDK/build/cmake/android.toolchain.cmake"

# Gradle reads the SDK location from here. Machine-specific, so not committed.
printf 'sdk.dir=%s\n' "$SDK" > "$ANDROID_DIR/local.properties"

# Keep every crate's artifacts in the repository target dir instead of leaving
# a nested target/ inside examples/.
export CARGO_TARGET_DIR="$ROOT/target"

# Android 15 requires 16 KB page alignment. NDK r28+ already defaults to it,
# but passing it explicitly costs nothing and keeps older NDKs working.
RUSTFLAGS="${RUSTFLAGS:-}"
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C link-arg=-Wl,-z,max-page-size=16384"
export RUSTFLAGS

abi_to_triple() {
    case "$1" in
        arm64-v8a) printf 'aarch64-linux-android\n' ;;
        armeabi-v7a) printf 'armv7-linux-androideabi\n' ;;
        x86_64) printf 'x86_64-linux-android\n' ;;
        x86) printf 'i686-linux-android\n' ;;
        *) echo "unsupported ABI: $1" >&2; return 1 ;;
    esac
}

TARGET_ARGS=()
TRIPLES=()
ABI_LIST=()
for abi in ${ABIS//,/ }; do
    abi="$(echo "$abi" | xargs)"
    [[ -z "$abi" ]] && continue
    triple="$(abi_to_triple "$abi")"
    TARGET_ARGS+=(-t "$abi")
    TRIPLES+=("$triple")
    ABI_LIST+=("$abi")
done

if [[ ${#TRIPLES[@]} -eq 0 ]]; then
    echo "no ABIs selected" >&2
    exit 1
fi

PROFILE_FLAG=()
if [[ "$PROFILE" == "release" ]]; then
    PROFILE_FLAG+=(--release)
fi

JNI_DIR="$ANDROID_DIR/app/src/main/jniLibs"
ASSETS_DIR="$ANDROID_DIR/app/src/main/assets"

echo "==> SDK:  $SDK"
echo "==> NDK:  $NDK"
echo "==> Game: $CRATE ($PROFILE)"
echo "==> ABIs: ${TRIPLES[*]}"

# --- Rust ----------------------------------------------------------------

if [[ ! -f "$ROOT/$CRATE/Cargo.toml" ]]; then
    echo "crate not found: $ROOT/$CRATE/Cargo.toml" >&2
    exit 1
fi

triple_to_abi() {
    case "$1" in
        aarch64-linux-android) printf 'arm64-v8a\n' ;;
        armv7-linux-androideabi) printf 'armeabi-v7a\n' ;;
        x86_64-linux-android) printf 'x86_64\n' ;;
        i686-linux-android) printf 'x86\n' ;;
        *) echo "unsupported triple: $1" >&2; return 1 ;;
    esac
}

mkdir -p "$JNI_DIR" "$ASSETS_DIR"
for triple in "${TRIPLES[@]}"; do
    abi="$(triple_to_abi "$triple")"
    mkdir -p "$JNI_DIR/$abi"
done

# SDLActivity takes the activity orientation from a runtime hint and overrides
# the manifest when the window is resizable. The Android crate reads this at
# compile time, so both places always agree with game.properties.
export FRGLIB_ORIENTATION="${ORIENTATION:-landscape}"

# cargo-ndk resolves the manifest from the current directory, so run it from the
# crate root. This also makes Cargo pick up <crate>/.cargo/config.toml.
pushd "$ROOT/$CRATE" >/dev/null
cargo ndk "${TARGET_ARGS[@]}" -P "$MIN_SDK" -o "$JNI_DIR" build "${PROFILE_FLAG[@]}"
popd >/dev/null

# --- Shared SDL libraries ------------------------------------------------

# sdl3-sys builds libSDL3.so next to the Rust artifacts, and sdl3-ttf-sys does
# the same for libSDL3_ttf.so. cargo-ndk only copies the crate's own cdylib, so
# the SDL libraries have to be picked up by hand.
for triple in "${TRIPLES[@]}"; do
    abi="$(triple_to_abi "$triple")"
    out="$CARGO_TARGET_DIR/$triple/$PROFILE"
    for lib in libSDL3.so libSDL3_ttf.so; do
        if [[ -f "$out/$lib" ]]; then
            cp -f "$out/$lib" "$JNI_DIR/$abi/"
        fi
    done
    if [[ ! -f "$JNI_DIR/$abi/libSDL3.so" ]]; then
        echo "warning: $out/libSDL3.so was not produced; SDL3 may be linked statically or missing" >&2
    fi
done

# --- Assets --------------------------------------------------------------

if [[ -n "$ASSETS" ]]; then
    if [[ ! -d "$ROOT/$ASSETS" ]]; then
        echo "assets directory not found: $ROOT/$ASSETS" >&2
        exit 1
    fi
    find "$ASSETS_DIR" -mindepth 1 ! -name '.gitkeep' -delete
    cp -R "$ROOT/$ASSETS/." "$ASSETS_DIR/"
    echo "==> Assets: $ASSETS -> app/src/main/assets"
fi

# --- Gradle --------------------------------------------------------------

TASK="${GRADLE_TASK_OVERRIDE:-assembleDebug}"
if [[ "$INSTALL" -eq 1 && -z "$GRADLE_TASK_OVERRIDE" ]]; then
    TASK="installDebug"
fi

GRADLE_ARGS=(
    "$TASK"
    "-PfrglibNdkVersion=$NDK_VERSION"
    "-PfrglibAbis=$(IFS=,; echo "${ABI_LIST[*]}")"
)
if [[ -n "$PACKAGE" ]]; then GRADLE_ARGS+=("-PfrglibPackage=$PACKAGE"); fi
if [[ -n "$APP_NAME" ]]; then GRADLE_ARGS+=("-PfrglibAppName=$APP_NAME"); fi
if [[ -n "$ORIENTATION" ]]; then GRADLE_ARGS+=("-PfrglibOrientation=$ORIENTATION"); fi

echo "==> Gradle: $TASK"
pushd "$ANDROID_DIR" >/dev/null
./gradlew "${GRADLE_ARGS[@]}"
popd >/dev/null

APK="$ANDROID_DIR/app/build/outputs/apk/debug/app-debug.apk"
if [[ -f "$APK" ]]; then
    echo "==> APK: $APK"
fi

if [[ "$INSTALL" -eq 1 ]]; then
    ADB="$SDK/platform-tools/adb"
    if [[ -x "$ADB" ]]; then
        echo "==> Launching $PACKAGE"
        "$ADB" shell am start -n "$PACKAGE/org.libsdl.app.SDLActivity" >/dev/null || true
    fi
fi
