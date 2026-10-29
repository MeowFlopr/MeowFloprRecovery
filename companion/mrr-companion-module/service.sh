#!/system/bin/sh
until [ "$(getprop sys.boot_completed)" = "1" ]; do
    sleep 3
done

SHIM=/mnt/vendor/persist/mrr/vold_retrieve_shim.so
if [ ! -f "$SHIM" ]; then
    SHIM=/data/local/tmp/vold_retrieve_shim.so
fi

if [ -f "$SHIM" ]; then
    RAW_KEYS=$(LD_PRELOAD="$SHIM" /system/bin/vold 2>&1 | grep 'RAW KEY' | awk '{print $NF}')
    DM_KEY=$(echo "$RAW_KEYS" | head -n 1)
    if [ -n "$DM_KEY" ] && [ ${#DM_KEY} -eq 128 ]; then
        mkdir -p /mnt/vendor/persist/mrr
        echo -n "$DM_KEY" > /mnt/vendor/persist/mrr/userdata.key
        chmod 600 /mnt/vendor/persist/mrr/userdata.key
    fi
fi
