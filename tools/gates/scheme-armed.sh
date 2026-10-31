#!/bin/sh
# Every log the Worker may append to is armed with the venue's id scheme (docs/design/MIGRATION-sha256-ids.md).
exec bash "$(dirname "$0")/../deploy/scheme-armed.sh"
