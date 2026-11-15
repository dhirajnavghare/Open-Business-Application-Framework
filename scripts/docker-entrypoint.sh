#!/bin/sh
set -e

# Automatically fix directory permissions for SQLite volume mounts if running as root
if [ "$(id -u)" = "0" ]; then
    mkdir -p /app/data
    chown -R obaf:obaf /app/data
    chmod 775 /app/data

    # If any database files already exist, ensure obaf ownership
    if [ -f /app/data/obaf.db ]; then
        chown obaf:obaf /app/data/obaf.db* 2>/dev/null || true
    fi

    # Drop root privileges and execute command as user obaf
    if command -v gosu >/dev/null 2>&1; then
        exec gosu obaf "$@"
    else
        exec su -s /bin/sh obaf -c "$*"
    fi
fi

exec "$@"
