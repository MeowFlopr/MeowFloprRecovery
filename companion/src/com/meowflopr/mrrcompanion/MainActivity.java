package com.meowflopr.mrrcompanion;

import android.app.Activity;
import android.content.res.ColorStateList;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.io.BufferedReader;
import java.io.DataOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.FileReader;
import java.io.InputStream;
import java.io.InputStreamReader;

public class MainActivity extends Activity {
    private TextView tvStatusTitle;
    private TextView tvStatusDesc;
    private TextView tvKeySnippet;
    private View statusIndicatorDot;
    private LinearLayout statusPill;
    private TextView tvStatusPillText;
    private TextView tvLog;
    private ScrollView svLog;
    private Handler mainHandler;

    private static final int COLOR_BG = Color.parseColor("#141218");
    private static final int COLOR_SURFACE_CONTAINER = Color.parseColor("#1D1B20");
    private static final int COLOR_SURFACE_HIGH = Color.parseColor("#211F26");
    private static final int COLOR_SURFACE_CARD = Color.parseColor("#2B2930");
    private static final int COLOR_PRIMARY = Color.parseColor("#D0BCFF");
    private static final int COLOR_ON_PRIMARY = Color.parseColor("#381E72");
    private static final int COLOR_PRIMARY_CONTAINER = Color.parseColor("#4F378B");
    private static final int COLOR_ON_PRIMARY_CONTAINER = Color.parseColor("#EADDFF");
    private static final int COLOR_SECONDARY_CONTAINER = Color.parseColor("#4A4458");
    private static final int COLOR_ON_SECONDARY_CONTAINER = Color.parseColor("#E8DEF8");
    private static final int COLOR_OUTLINE = Color.parseColor("#938F99");
    private static final int COLOR_OUTLINE_VARIANT = Color.parseColor("#49454F");
    private static final int COLOR_ON_SURFACE = Color.parseColor("#E6E0E9");
    private static final int COLOR_ON_SURFACE_VAR = Color.parseColor("#CAC4D0");
    private static final int COLOR_SUCCESS = Color.parseColor("#7CE6A2");
    private static final int COLOR_SUCCESS_CONTAINER = Color.parseColor("#00522B");
    private static final int COLOR_ERROR = Color.parseColor("#FFB4AB");
    private static final int COLOR_ERROR_CONTAINER = Color.parseColor("#93000A");

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        mainHandler = new Handler(Looper.getMainLooper());

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(COLOR_BG);
        root.setPadding(40, 60, 40, 30);

        LinearLayout topBar = new LinearLayout(this);
        topBar.setOrientation(LinearLayout.VERTICAL);
        topBar.setPadding(0, 0, 0, 24);

        LinearLayout titleRow = new LinearLayout(this);
        titleRow.setOrientation(LinearLayout.HORIZONTAL);
        titleRow.setGravity(Gravity.CENTER_VERTICAL);

        TextView tvTitle = new TextView(this);
        tvTitle.setText("MeowFloprRecovery");
        tvTitle.setTextSize(26);
        tvTitle.setTextColor(COLOR_ON_SURFACE);
        tvTitle.setTypeface(null, Typeface.BOLD);
        titleRow.addView(tvTitle);

        TextView tvBadge = new TextView(this);
        tvBadge.setText("v2.5");
        tvBadge.setTextSize(12);
        tvBadge.setTextColor(COLOR_ON_PRIMARY_CONTAINER);
        tvBadge.setTypeface(null, Typeface.BOLD);
        tvBadge.setBackground(createPillDrawable(COLOR_PRIMARY_CONTAINER, 20, 0, 0));
        tvBadge.setPadding(20, 6, 20, 6);
        LinearLayout.LayoutParams pBadge = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        pBadge.setMargins(20, 0, 0, 0);
        tvBadge.setLayoutParams(pBadge);
        titleRow.addView(tvBadge);

        topBar.addView(titleRow);

        TextView tvSub = new TextView(this);
        tvSub.setText("Xiaomi 14T (degas) • KeyMint FBE v2 Hardware Sync");
        tvSub.setTextSize(13);
        tvSub.setTextColor(COLOR_OUTLINE);
        tvSub.setPadding(0, 6, 0, 0);
        topBar.addView(tvSub);

        root.addView(topBar);

        LinearLayout heroCard = new LinearLayout(this);
        heroCard.setOrientation(LinearLayout.VERTICAL);
        heroCard.setBackground(createPillDrawable(COLOR_SURFACE_HIGH, 36, COLOR_OUTLINE_VARIANT, 2));
        heroCard.setPadding(36, 32, 36, 32);

        LinearLayout statusHeaderRow = new LinearLayout(this);
        statusHeaderRow.setOrientation(LinearLayout.HORIZONTAL);
        statusHeaderRow.setGravity(Gravity.CENTER_VERTICAL);

        tvStatusTitle = new TextView(this);
        tvStatusTitle.setText("Storage Encryption");
        tvStatusTitle.setTextSize(18);
        tvStatusTitle.setTextColor(COLOR_ON_SURFACE);
        tvStatusTitle.setTypeface(null, Typeface.BOLD);
        LinearLayout.LayoutParams pStatusTitle = new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1.0f);
        tvStatusTitle.setLayoutParams(pStatusTitle);
        statusHeaderRow.addView(tvStatusTitle);

        statusPill = new LinearLayout(this);
        statusPill.setOrientation(LinearLayout.HORIZONTAL);
        statusPill.setGravity(Gravity.CENTER_VERTICAL);
        statusPill.setBackground(createPillDrawable(COLOR_SURFACE_CARD, 24, COLOR_OUTLINE_VARIANT, 1));
        statusPill.setPadding(20, 8, 20, 8);

        statusIndicatorDot = new View(this);
        GradientDrawable dotDrawable = new GradientDrawable();
        dotDrawable.setShape(GradientDrawable.OVAL);
        dotDrawable.setColor(COLOR_OUTLINE);
        statusIndicatorDot.setBackground(dotDrawable);
        LinearLayout.LayoutParams pDot = new LinearLayout.LayoutParams(16, 16);
        pDot.setMargins(0, 0, 14, 0);
        statusIndicatorDot.setLayoutParams(pDot);
        statusPill.addView(statusIndicatorDot);

        tvStatusPillText = new TextView(this);
        tvStatusPillText.setText("CHECKING");
        tvStatusPillText.setTextSize(11);
        tvStatusPillText.setTextColor(COLOR_OUTLINE);
        tvStatusPillText.setTypeface(null, Typeface.BOLD);
        statusPill.addView(tvStatusPillText);

        statusHeaderRow.addView(statusPill);
        heroCard.addView(statusHeaderRow);

        tvStatusDesc = new TextView(this);
        tvStatusDesc.setText("Probing hardware encryption state from persist...");
        tvStatusDesc.setTextSize(13);
        tvStatusDesc.setTextColor(COLOR_ON_SURFACE_VAR);
        tvStatusDesc.setPadding(0, 16, 0, 16);
        heroCard.addView(tvStatusDesc);

        LinearLayout keyBox = new LinearLayout(this);
        keyBox.setOrientation(LinearLayout.VERTICAL);
        keyBox.setBackground(createPillDrawable(COLOR_BG, 20, COLOR_OUTLINE_VARIANT, 1));
        keyBox.setPadding(24, 20, 24, 20);

        TextView tvKeyLabel = new TextView(this);
        tvKeyLabel.setText("PERSIST KEY STATUS");
        tvKeyLabel.setTextSize(10);
        tvKeyLabel.setTextColor(COLOR_OUTLINE);
        tvKeyLabel.setTypeface(null, Typeface.BOLD);
        keyBox.addView(tvKeyLabel);

        tvKeySnippet = new TextView(this);
        tvKeySnippet.setText("/mnt/vendor/persist/mrr/userdata.key: Reading...");
        tvKeySnippet.setTextSize(11);
        tvKeySnippet.setTextColor(COLOR_PRIMARY);
        tvKeySnippet.setTypeface(Typeface.MONOSPACE);
        tvKeySnippet.setPadding(0, 6, 0, 0);
        keyBox.addView(tvKeySnippet);

        heroCard.addView(keyBox);
        root.addView(heroCard);

        LinearLayout actionsLayout = new LinearLayout(this);
        actionsLayout.setOrientation(LinearLayout.VERTICAL);
        actionsLayout.setPadding(0, 28, 0, 20);

        Button btnSync = createM3Button("Sync Hardware Keys Now", COLOR_PRIMARY, COLOR_ON_PRIMARY, 0, 0);
        btnSync.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                runSyncKeys();
            }
        });
        actionsLayout.addView(btnSync);

        Button btnInstallService = createM3Button("Install Boot Auto-Sync Module", COLOR_SECONDARY_CONTAINER, COLOR_ON_SECONDARY_CONTAINER, 0, 0);
        LinearLayout.LayoutParams pBtn2 = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        pBtn2.setMargins(0, 16, 0, 0);
        btnInstallService.setLayoutParams(pBtn2);
        btnInstallService.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                installBootService();
            }
        });
        actionsLayout.addView(btnInstallService);

        Button btnRebootRec = createM3Button("Reboot to MrR Recovery", Color.TRANSPARENT, COLOR_PRIMARY, COLOR_OUTLINE, 2);
        LinearLayout.LayoutParams pBtn3 = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        pBtn3.setMargins(0, 16, 0, 0);
        btnRebootRec.setLayoutParams(pBtn3);
        btnRebootRec.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                rebootRecovery();
            }
        });
        actionsLayout.addView(btnRebootRec);

        root.addView(actionsLayout);

        LinearLayout consoleCard = new LinearLayout(this);
        consoleCard.setOrientation(LinearLayout.VERTICAL);
        consoleCard.setBackground(createPillDrawable(Color.parseColor("#0E0E12"), 28, COLOR_OUTLINE_VARIANT, 1));
        consoleCard.setPadding(30, 22, 30, 22);
        LinearLayout.LayoutParams pConsole = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 0, 1.0f);
        consoleCard.setLayoutParams(pConsole);

        LinearLayout consoleHeader = new LinearLayout(this);
        consoleHeader.setOrientation(LinearLayout.HORIZONTAL);
        consoleHeader.setGravity(Gravity.CENTER_VERTICAL);
        consoleHeader.setPadding(0, 0, 0, 14);

        TextView tvConsoleTitle = new TextView(this);
        tvConsoleTitle.setText("Execution Console");
        tvConsoleTitle.setTextSize(13);
        tvConsoleTitle.setTextColor(COLOR_OUTLINE);
        tvConsoleTitle.setTypeface(null, Typeface.BOLD);
        LinearLayout.LayoutParams pConTitle = new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1.0f);
        tvConsoleTitle.setLayoutParams(pConTitle);
        consoleHeader.addView(tvConsoleTitle);

        Button btnClear = new Button(this);
        btnClear.setText("Clear");
        btnClear.setTextSize(11);
        btnClear.setTextColor(COLOR_ON_SECONDARY_CONTAINER);
        btnClear.setBackground(createM3ButtonDrawable(COLOR_SECONDARY_CONTAINER, 16, 0, 0));
        btnClear.setPadding(24, 4, 24, 4);
        btnClear.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                tvLog.setText("");
            }
        });
        consoleHeader.addView(btnClear);

        consoleCard.addView(consoleHeader);

        svLog = new ScrollView(this);
        LinearLayout.LayoutParams svParams = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.MATCH_PARENT);
        svLog.setLayoutParams(svParams);

        tvLog = new TextView(this);
        tvLog.setText("Ready. Press 'Sync Hardware Keys Now' to extract KeyMint blob.\n");
        tvLog.setTextSize(11);
        tvLog.setTextColor(Color.parseColor("#A6ACB8"));
        tvLog.setTypeface(Typeface.MONOSPACE);
        svLog.addView(tvLog);

        consoleCard.addView(svLog);
        root.addView(consoleCard);

        setContentView(root);

        extractShimAsset();
        checkStatus();
    }

    private Button createM3Button(String text, int bgColor, int textColor, int strokeColor, int strokeWidth) {
        Button btn = new Button(this);
        btn.setText(text);
        btn.setTextSize(14);
        btn.setTextColor(textColor);
        btn.setTypeface(null, Typeface.BOLD);
        btn.setBackground(createM3ButtonDrawable(bgColor, 32, strokeColor, strokeWidth));
        btn.setPadding(0, 32, 0, 32);
        btn.setLayoutParams(new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));
        return btn;
    }

    private GradientDrawable createPillDrawable(int bgColor, int cornerRadius, int strokeColor, int strokeWidth) {
        GradientDrawable gd = new GradientDrawable();
        gd.setShape(GradientDrawable.RECTANGLE);
        gd.setColor(bgColor);
        gd.setCornerRadius(cornerRadius);
        if (strokeWidth > 0 && strokeColor != 0) {
            gd.setStroke(strokeWidth, strokeColor);
        }
        return gd;
    }

    private RippleDrawable createM3ButtonDrawable(int normalColor, int cornerRadius, int strokeColor, int strokeWidth) {
        GradientDrawable normal = createPillDrawable(normalColor, cornerRadius, strokeColor, strokeWidth);
        int rippleColor = Color.argb(40, 255, 255, 255);
        return new RippleDrawable(ColorStateList.valueOf(rippleColor), normal, null);
    }

    private void log(final String msg) {
        mainHandler.post(new Runnable() {
            @Override
            public void run() {
                tvLog.append(msg + "\n");
                svLog.post(new Runnable() {
                    @Override
                    public void run() {
                        svLog.fullScroll(ScrollView.FOCUS_DOWN);
                    }
                });
            }
        });
    }

    private void extractShimAsset() {
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    File out = new File(getFilesDir(), "vold_retrieve_shim.so");
                    InputStream in = getAssets().open("vold_retrieve_shim.so");
                    FileOutputStream fos = new FileOutputStream(out);
                    byte[] buf = new byte[8192];
                    int r;
                    while ((r = in.read(buf)) != -1) {
                        fos.write(buf, 0, r);
                    }
                    fos.flush();
                    fos.close();
                    in.close();
                    out.setReadable(true, false);
                    out.setExecutable(true, false);
                } catch (Exception ignored) {
                }
            }
        }).start();
    }

    private void checkStatus() {
        new Thread(new Runnable() {
            @Override
            public void run() {
                File keyFile = new File("/mnt/vendor/persist/mrr/userdata.key");
                boolean keyExists = false;
                String snippet = "Not found on persist partition";

                if (keyFile.exists() && keyFile.length() >= 128) {
                    try {
                        BufferedReader br = new BufferedReader(new FileReader(keyFile));
                        String line = br.readLine();
                        br.close();
                        if (line != null && line.trim().length() == 128) {
                            keyExists = true;
                            String trimmed = line.trim();
                            snippet = trimmed.substring(0, 16) + "..." + trimmed.substring(112) + " (128 hex)";
                        }
                    } catch (Exception ignored) {
                    }
                }

                final boolean finalKey = keyExists;
                final String finalSnippet = snippet;

                mainHandler.post(new Runnable() {
                    @Override
                    public void run() {
                        if (finalKey) {
                            tvStatusDesc.setText("Master AES-256-XTS key active in persist. MrR Recovery will auto-decrypt data seamlessly.");
                            tvKeySnippet.setText(finalSnippet);
                            tvKeySnippet.setTextColor(COLOR_SUCCESS);

                            GradientDrawable dot = new GradientDrawable();
                            dot.setShape(GradientDrawable.OVAL);
                            dot.setColor(COLOR_SUCCESS);
                            statusIndicatorDot.setBackground(dot);

                            statusPill.setBackground(createPillDrawable(COLOR_SUCCESS_CONTAINER, 24, COLOR_SUCCESS, 1));
                            tvStatusPillText.setText("ACTIVE");
                            tvStatusPillText.setTextColor(COLOR_SUCCESS);
                        } else {
                            tvStatusDesc.setText("No hardware key stored in persist. Press 'Sync Hardware Keys Now' below to unseal.");
                            tvKeySnippet.setText(finalSnippet);
                            tvKeySnippet.setTextColor(COLOR_ERROR);

                            GradientDrawable dot = new GradientDrawable();
                            dot.setShape(GradientDrawable.OVAL);
                            dot.setColor(COLOR_ERROR);
                            statusIndicatorDot.setBackground(dot);

                            statusPill.setBackground(createPillDrawable(COLOR_ERROR_CONTAINER, 24, COLOR_ERROR, 1));
                            tvStatusPillText.setText("NOT SYNCED");
                            tvStatusPillText.setTextColor(COLOR_ERROR);
                        }
                    }
                });
            }
        }).start();
    }

    private void runSyncKeys() {
        log("[*] Requesting root privileges and unsealing hardware key from KeyMint...");
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    File shimFile = new File(getFilesDir(), "vold_retrieve_shim.so");
                    if (!shimFile.exists() || shimFile.length() == 0) {
                        extractShimAsset();
                    }

                    Process su = Runtime.getRuntime().exec("su");
                    DataOutputStream dos = new DataOutputStream(su.getOutputStream());
                    BufferedReader reader = new BufferedReader(new InputStreamReader(su.getInputStream()));
                    BufferedReader errReader = new BufferedReader(new InputStreamReader(su.getErrorStream()));

                    dos.writeBytes("mkdir -p /mnt/vendor/persist/mrr\n");
                    dos.writeBytes("chmod 777 /mnt/vendor/persist/mrr\n");
                    dos.writeBytes("cp " + shimFile.getAbsolutePath() + " /data/local/tmp/vold_retrieve_shim.so\n");
                    dos.writeBytes("cp " + shimFile.getAbsolutePath() + " /mnt/vendor/persist/mrr/vold_retrieve_shim.so\n");
                    dos.writeBytes("chmod 755 /data/local/tmp/vold_retrieve_shim.so\n");
                    dos.writeBytes("chmod 755 /mnt/vendor/persist/mrr/vold_retrieve_shim.so\n");
                    dos.writeBytes("LD_PRELOAD=/data/local/tmp/vold_retrieve_shim.so /system/bin/vold 2>&1\n");
                    dos.writeBytes("echo ___CMD_DONE___ $?\n");
                    dos.flush();

                    String line;
                    String extractedKey = null;
                    while ((line = reader.readLine()) != null) {
                        if (line.contains("___CMD_DONE___")) {
                            break;
                        }
                        log("[vold] " + line);
                        if (line.contains("RAW KEY (HEX):")) {
                            String[] parts = line.split("RAW KEY \\(HEX\\):");
                            if (parts.length > 1) {
                                extractedKey = parts[1].trim();
                            }
                        }
                    }

                    if (extractedKey != null && extractedKey.length() == 128) {
                        log("[+] Hardware Key successfully unsealed: " + extractedKey.substring(0, 16) + "...");
                        dos.writeBytes("echo -n " + extractedKey + " > /mnt/vendor/persist/mrr/userdata.key\n");
                        dos.writeBytes("chmod 600 /mnt/vendor/persist/mrr/userdata.key\n");
                        dos.writeBytes("echo ___WRITE_DONE___ $?\n");
                        dos.flush();

                        while ((line = reader.readLine()) != null) {
                            if (line.contains("___WRITE_DONE___")) {
                                break;
                            }
                        }
                        log("[+] Saved to /mnt/vendor/persist/mrr/userdata.key (128 hex chars, chmod 600)");
                        log("[SUCCESS] Keys Synced Successfully! You can now reboot to MrR Recovery.");
                    } else {
                        log("[-] Error: Could not unseal 128-char AES-256-XTS key.");
                    }

                    dos.writeBytes("exit\n");
                    dos.flush();
                    su.waitFor();
                    checkStatus();
                } catch (Exception e) {
                    log("[ERR] Execution failed: " + e.getMessage());
                }
            }
        }).start();
    }

    private void installBootService() {
        log("[*] Installing Magisk/KernelSU auto-sync boot module...");
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    Process su = Runtime.getRuntime().exec("su");
                    DataOutputStream dos = new DataOutputStream(su.getOutputStream());
                    BufferedReader reader = new BufferedReader(new InputStreamReader(su.getInputStream()));

                    dos.writeBytes("MOD=/data/adb/modules/mrr-companion\n");
                    dos.writeBytes("mkdir -p $MOD\n");
                    dos.writeBytes("touch $MOD/auto_mount\n");
                    dos.writeBytes("echo 'id=mrr-companion' > $MOD/module.prop\n");
                    dos.writeBytes("echo 'name=MeowFloprRecovery Companion' >> $MOD/module.prop\n");
                    dos.writeBytes("echo 'version=v2.5' >> $MOD/module.prop\n");
                    dos.writeBytes("echo 'versionCode=250' >> $MOD/module.prop\n");
                    dos.writeBytes("echo 'author=MeowFlopr' >> $MOD/module.prop\n");
                    dos.writeBytes("echo 'description=Syncs MT6897 FBE hardware keys on boot.' >> $MOD/module.prop\n");

                    dos.writeBytes("cat << 'EOF' > $MOD/service.sh\n");
                    dos.writeBytes("#!/system/bin/sh\n");
                    dos.writeBytes("until [ \"$(getprop sys.boot_completed)\" = \"1\" ]; do sleep 3; done\n");
                    dos.writeBytes("SHIM=/mnt/vendor/persist/mrr/vold_retrieve_shim.so\n");
                    dos.writeBytes("if [ -f \"$SHIM\" ]; then\n");
                    dos.writeBytes("  RAW=$(LD_PRELOAD=\"$SHIM\" /system/bin/vold 2>&1 | grep 'RAW KEY' | awk '{print $NF}')\n");
                    dos.writeBytes("  if [ ${#RAW} -eq 128 ]; then\n");
                    dos.writeBytes("    echo -n \"$RAW\" > /mnt/vendor/persist/mrr/userdata.key\n");
                    dos.writeBytes("    chmod 600 /mnt/vendor/persist/mrr/userdata.key\n");
                    dos.writeBytes("  fi\n");
                    dos.writeBytes("fi\n");
                    dos.writeBytes("EOF\n");

                    dos.writeBytes("chmod 755 $MOD/service.sh\n");
                    dos.writeBytes("echo ___INSTALL_DONE___ $?\n");
                    dos.flush();

                    String line;
                    while ((line = reader.readLine()) != null) {
                        if (line.contains("___INSTALL_DONE___")) {
                            break;
                        }
                    }

                    dos.writeBytes("exit\n");
                    dos.flush();
                    su.waitFor();

                    log("[+] Module installed to /data/adb/modules/mrr-companion/");
                    log("[+] Keys will now automatically sync at every system boot!");
                } catch (Exception e) {
                    log("[ERR] Module install failed: " + e.getMessage());
                }
            }
        }).start();
    }

    private void rebootRecovery() {
        log("[*] Requesting reboot to recovery mode...");
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    Process su = Runtime.getRuntime().exec("su");
                    DataOutputStream dos = new DataOutputStream(su.getOutputStream());
                    dos.writeBytes("/system/bin/reboot recovery\n");
                    dos.flush();
                } catch (Exception e) {
                    log("[ERR] Reboot failed: " + e.getMessage());
                }
            }
        }).start();
    }
}
