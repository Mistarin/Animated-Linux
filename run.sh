#!/usr/bin/env bash
# Copyright (C) 2026 Animated Linux Contributors
# Licensed under GNU GPL-3.0

set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$DIR/setup/launch.sh" "$@"
