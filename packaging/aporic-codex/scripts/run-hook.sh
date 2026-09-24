#!/bin/sh
set -eu

aporic_data_root=${APORIC_DATA_HOME:-${PLUGIN_DATA}}
exec "${PLUGIN_ROOT}/bin/aporicctl" codex-hook "${aporic_data_root}/catalog"
