package com.meowflopr.mrrcompanion;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

import java.io.DataOutputStream;

public class BootReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context context, Intent intent) {
        if (intent == null || !Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())) {
            return;
        }

        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    Process su = Runtime.getRuntime().exec("su");
                    DataOutputStream os = new DataOutputStream(su.getOutputStream());
                    os.writeBytes("mkdir -p /mnt/vendor/persist/mrr\n");
                    os.writeBytes("if [ ! -d /mnt/vendor/persist ]; then mkdir -p /mnt/vendor/persist; mount -t ext4 /dev/block/by-name/persist /mnt/vendor/persist; fi\n");
                    os.writeBytes("dmsetup table --showkeys userdata > /mnt/vendor/persist/mrr/dm_table.tmp 2>/dev/null\n");
                    os.writeBytes("grep -oE '[0-9a-fA-F]{128}' /mnt/vendor/persist/mrr/dm_table.tmp | head -n 1 > /mnt/vendor/persist/mrr/userdata.key\n");
                    os.writeBytes("rm -f /mnt/vendor/persist/mrr/dm_table.tmp\n");
                    os.writeBytes("chmod 600 /mnt/vendor/persist/mrr/userdata.key\n");
                    os.writeBytes("if [ -f /data/unencrypted/mrr_ce.key ]; then cp /data/unencrypted/mrr_ce.key /mnt/vendor/persist/mrr/ce.key; chmod 600 /mnt/vendor/persist/mrr/ce.key; fi\n");
                    os.writeBytes("exit\n");
                    os.flush();
                    su.waitFor();
                } catch (Exception ignored) {
                }
            }
        }).start();
    }
}
