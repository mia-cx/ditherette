#!/bin/sh
root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 1
export WEBKIT_EXEC_PATH="$root/engine/minibrowser-wpe/bin"
export WEBKIT_INJECTED_BUNDLE_PATH="$root/engine/minibrowser-wpe/lib"
export WEBKIT_INSPECTOR_RESOURCES_PATH="$root/engine/minibrowser-wpe/share"
export LD_LIBRARY_PATH="$root/libraries:$root/engine/minibrowser-wpe/lib:$root/engine/minibrowser-wpe/sys/lib"
export WEBKIT_FORCE_COMPLEX_TEXT=1
exec "$root/engine/minibrowser-wpe/bin/MiniBrowser" "$@"
