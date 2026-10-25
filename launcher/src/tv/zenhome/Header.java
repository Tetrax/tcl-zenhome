package tv.zenhome;

import android.app.ActivityManager;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.Environment;
import android.graphics.drawable.GradientDrawable;
import android.os.StatFs;
import android.text.TextUtils;
import android.text.format.DateFormat;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ImageView;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

/** Salutation + phrase d'ambiance, horloge, date, RAM / stockage, et les ticks qui les rafraichissent. */
class Header {
    final Context ctx;
    final Ui ui;
    final SharedPreferences prefs;
    final Async async;
    final HomeConfig config;
    final Host host;
    TextView clock, date, titleView, phraseView;   // titleView = salutation
    LinearLayout greetRow;
    LinearLayout panel;    // panneau en verre sombre : jauges, separateur, puis la rangee de raccourcis (ajoutee par Shortcuts)
    LinearLayout gauges;   // RAM + disque
    View sep;
    TextView ramText, diskText;
    View ramBlock, diskBlock;
    Icons.RingGauge ramRing = new Icons.RingGauge(), diskRing = new Icons.RingGauge();
    LinearLayout col;   // colonne en haut a droite : horloge, date, RAM (et les raccourcis, ajoutes par Shortcuts)
    String greetShown = "", phraseShown = "";

    Header(Context ctx, Ui ui, SharedPreferences prefs, Async async, HomeConfig config, Host host) {
        this.ctx = ctx; this.ui = ui; this.prefs = prefs; this.async = async; this.config = config; this.host = host;
    }

    /** Salutation selon l'heure + phrase d'ambiance sur la meme ligne (900 unites max : pas de contact avec l'horloge). */
    void addGreeting(ViewGroup parent) {
        greetRow = new LinearLayout(ctx);
        greetRow.setOrientation(LinearLayout.HORIZONTAL);
        greetRow.setGravity(Gravity.BOTTOM);
        titleView = ui.text("", 26, ui.fMed, 0xFFFFFFFF);
        titleView.setSingleLine(true);
        titleView.setEllipsize(TextUtils.TruncateAt.END);
        titleView.setMaxWidth(ui.dp(560));
        greetRow.addView(titleView, new LinearLayout.LayoutParams(-2, -2));
        phraseView = ui.text("", 14, ui.fReg, 0xCCFFFFFF);
        phraseView.setSingleLine(true);
        phraseView.setEllipsize(TextUtils.TruncateAt.END);
        phraseView.setPadding(ui.dp(12), 0, 0, ui.dp(4));
        greetRow.addView(phraseView, new LinearLayout.LayoutParams(0, -2, 1f));
        FrameLayout.LayoutParams lpT = new FrameLayout.LayoutParams(ui.dp(900), -2, Gravity.TOP | Gravity.START);
        lpT.setMargins(ui.dp(56), ui.dp(38), 0, 0);
        parent.addView(greetRow, lpT);
        applyTitle();
    }

    /** Horloge + date + RAM. */
    void addClock(ViewGroup parent) {
        col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setGravity(Gravity.END);
        clock = ui.text("", 40, ui.fLight, 0xFFFFFFFF);
        date = ui.text("", 15, ui.fReg, 0xE6FFFFFF);
        col.addView(clock); col.addView(date);
        panel = new LinearLayout(ctx);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setClipChildren(false);
        panel.setPadding(0, ui.dp(6), 0, 0);
        GradientDrawable pbg = new GradientDrawable(); pbg.setCornerRadius(ui.dp(22)); ui.theme.bind(pbg, Theme.PANEL, Math.max(1, ui.dp(0.75f)));
        panel.setBackground(pbg);
        gauges = new LinearLayout(ctx);   // [anneau] RAM 73 %   [anneau] Disque 16 %, repartis sur la largeur de la rangee de boutons
        gauges.setGravity(Gravity.CENTER_VERTICAL);
        gauges.setPadding(ui.dp(10), ui.dp(2), ui.dp(10), ui.dp(4));
        TextView[] t = new TextView[1];
        ramBlock = gaugeBlock(ui.t("RAM", "RAM"), ramRing, t); ramText = t[0];
        diskBlock = gaugeBlock(ui.t("Disque", "Disk"), diskRing, t); diskText = t[0];
        gauges.addView(ramBlock); gauges.addView(diskBlock);
        gauges.setVisibility(View.GONE);
        sep = new View(ctx);
        sep.setBackgroundColor(0x26FFFFFF);
        sep.setVisibility(View.GONE);
        // jauges et separateur en MATCH_PARENT : la largeur du panneau est celle de la rangee de boutons (seul enfant WRAP_CONTENT)
        panel.addView(gauges, new LinearLayout.LayoutParams(-1, -2));
        LinearLayout.LayoutParams lpS = new LinearLayout.LayoutParams(-1, 1); lpS.setMargins(ui.dp(14), 0, ui.dp(14), 0);
        panel.addView(sep, lpS);
        LinearLayout.LayoutParams lpG = new LinearLayout.LayoutParams(-2, -2); lpG.topMargin = ui.dp(6);
        col.addView(panel, lpG);
        FrameLayout.LayoutParams lpC = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.END);
        lpC.setMargins(0, ui.dp(26), ui.dp(56), 0);
        parent.addView(col, lpC);
    }

    /** Un bloc : anneau, puis libelle au-dessus du pourcentage. out[0] recoit le TextView du pourcentage. */
    View gaugeBlock(String label, Icons.RingGauge ring, TextView[] out) {
        LinearLayout b = new LinearLayout(ctx);
        b.setGravity(Gravity.CENTER);
        ImageView iv = new ImageView(ctx); iv.setImageDrawable(ring);
        b.addView(iv, new LinearLayout.LayoutParams(ui.dp(28), ui.dp(28)));
        LinearLayout tx = new LinearLayout(ctx);
        tx.setOrientation(LinearLayout.VERTICAL);
        tx.setPadding(ui.dp(7), 0, 0, 0);
        TextView lb = ui.text(label, 9, ui.fMed, 0xCCFFFFFF);
        TextView pc = ui.text("", 14, ui.fMed, 0xFFFFFFFF);
        tx.addView(lb); tx.addView(pc);
        b.addView(tx);
        b.setLayoutParams(new LinearLayout.LayoutParams(0, -2, 1f));
        out[0] = pc;
        return b;
    }

    void applyTitle() {
        greetRow.animate().cancel(); greetRow.setAlpha(1f);
        greetShown = ""; phraseShown = "";
        updateGreeting();
    }

    // ---------- Salutation + phrase d'ambiance (deterministes : graine = jour de l'annee et tranche horaire)

    String phrase(java.util.Calendar c, int slot) {
        return Greeting.phrase(c.get(java.util.Calendar.DAY_OF_YEAR), c.get(java.util.Calendar.DAY_OF_WEEK), slot,
                host.weatherCode(), prefs.getString("city", ""), ui.fr);
    }

    /** Met a jour la ligne de salutation ; le texte ne change (en fondu) que si salutation ou phrase change. */
    void updateGreeting() {
        if ("none".equals(config.titleMode)) { greetRow.setVisibility(View.GONE); greetShown = ""; phraseShown = ""; return; }
        java.util.Calendar c = java.util.Calendar.getInstance();
        int slot = Greeting.slotOf(c.get(java.util.Calendar.HOUR_OF_DAY));
        String g = Greeting.GREET[slot][ui.fr ? 0 : 1];
        if ("custom".equals(config.titleMode) && !config.titleText.isEmpty()) g += ", " + config.titleText;
        String ph = phrase(c, slot);
        if (g.equals(greetShown) && ph.equals(phraseShown)) return;
        boolean first = greetShown.isEmpty();
        greetShown = g; phraseShown = ph;
        titleView.setText(g);
        phraseView.setText(ph.isEmpty() ? "" : "·  " + ph);
        greetRow.setVisibility(View.VISIBLE);
        if (!first) { greetRow.setAlpha(0f); greetRow.animate().alpha(1f).setDuration(400).start(); }
    }

    /** Heure affichee (24 h ou 12 h selon le reglage du systeme) : partagee avec l'ambiance et l'economiseur. */
    static CharSequence time(Context ctx, Date n) {
        return DateFormat.is24HourFormat(ctx) ? DateFormat.format("HH:mm", n) : DateFormat.format("h:mm", n);
    }

    static String dateText(Ui ui, Date n) {
        Locale loc = Locale.getDefault();
        String d = new SimpleDateFormat(ui.fr ? "EEEE d MMMM" : "EEEE, MMMM d", loc).format(n);
        return d.substring(0, 1).toUpperCase(loc) + d.substring(1);
    }

    final Runnable tick = new Runnable() {
        public void run() {
            Date n = new Date();
            clock.setText(time(ctx, n));
            date.setText(dateText(ui, n));
            updateGreeting();
            host.dataTick();
            async.h.postDelayed(this, 15_000);
        }
    };

    // RAM + stockage (toutes les 5 s, uniquement au premier plan : memTick est dans h)
    final Runnable memTick = new Runnable() {
        public void run() { updateMem(); async.h.postDelayed(this, 5000); }
    };

    /** Lance l'horloge et la RAM (onResume, apres le vidage de async.h). */
    void start() {
        async.h.post(tick);
        async.h.post(memTick);
    }

    void updateMem() {
        long total = -1, avail = -1;
        java.io.BufferedReader br = null;
        try {
            br = new java.io.BufferedReader(new java.io.FileReader("/proc/meminfo"));
            String l;
            while ((l = br.readLine()) != null && (total < 0 || avail < 0)) {
                if (l.startsWith("MemTotal:")) total = Fmt.kb(l);
                else if (l.startsWith("MemAvailable:")) avail = Fmt.kb(l);
            }
        } catch (Exception ignored) { }
        finally { try { if (br != null) br.close(); } catch (Exception ignored) { } }
        if (total <= 0 || avail < 0) {
            try {
                ActivityManager.MemoryInfo mi = new ActivityManager.MemoryInfo();
                ((ActivityManager) ctx.getSystemService(Context.ACTIVITY_SERVICE)).getMemoryInfo(mi);
                total = mi.totalMem / 1024; avail = mi.availMem / 1024;
            } catch (Exception ignored) { }
        }
        int ram = GaugeRules.pct(total - avail, total), disk = -1;   // RAM occupee = total - disponible (cache libérable exclu)
        try {
            StatFs sf = new StatFs(Environment.getDataDirectory().getPath());
            double bs = sf.getBlockSizeLong(), tot = sf.getBlockCountLong() * bs;
            disk = GaugeRules.pct(tot - sf.getAvailableBlocksLong() * bs, tot);
        } catch (Exception ignored) { }
        ramRing.set(ram); diskRing.set(disk);
        ramText.setText(GaugeRules.text(ram, ui.fr)); diskText.setText(GaugeRules.text(disk, ui.fr));
        ramBlock.setVisibility(ram < 0 ? View.GONE : View.VISIBLE); diskBlock.setVisibility(disk < 0 ? View.GONE : View.VISIBLE);
        int gv = ram < 0 && disk < 0 ? View.GONE : View.VISIBLE;
        gauges.setVisibility(gv); sep.setVisibility(gv);
    }
}
