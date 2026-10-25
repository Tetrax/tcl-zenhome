package tv.zenhome;

import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;

import java.util.ArrayList;
import java.util.List;

/**
 * Minuteur de mise en veille. L'echeance (heure absolue) est dans les prefs (sleep_at, sleep_min) et dans deux alarmes
 * (avertissement 1 min avant, veille) livrees a SleepReceiver (non exporte) : elles survivent au lancement d'une appli.
 * Pas de permission SCHEDULE_EXACT_ALARM : alarme exacte seulement si le systeme l'autorise deja, sinon approximative.
 */
class Sleep {
    static final String A_SLEEP = "tv.zenhome.SLEEP", A_WARN = "tv.zenhome.SLEEP_WARN", EXTRA_AT = "at";

    final Context ctx;
    final Ui ui;
    final SharedPreferences prefs;
    final Overlays ovs;
    final Runnable changed;   // minuteur regle / annule : rafraichit le badge du raccourci

    Sleep(Context ctx, Ui ui, SharedPreferences prefs, Overlays ovs, Runnable changed) {
        this.ctx = ctx; this.ui = ui; this.prefs = prefs; this.ovs = ovs; this.changed = changed;
    }

    /** Temps restant pour le badge du raccourci (« 42 min », « 1 h 05 »), vide si aucun minuteur. */
    String badgeText() { return SleepRules.shortLeft(prefs.getLong("sleep_at", 0), System.currentTimeMillis()); }

    /** Au demarrage (la TV a pu redemarrer : les alarmes sont perdues) : echeance passee ignoree, sinon alarmes replanifiees. */
    void restore() {
        long d = prefs.getLong("sleep_at", 0), now = System.currentTimeMillis();
        if (d == 0) return;
        if (SleepRules.active(d, now)) arm(ctx, d, now);
        else clear();
    }

    void show() {
        ovs.showMenu(new MenuPanel.Source() { public MenuPanel.Page build() {
            long d = prefs.getLong("sleep_at", 0), now = System.currentTimeMillis();
            boolean on = SleepRules.active(d, now);
            MenuPanel.Page p = new MenuPanel.Page(ui.t("Mise en veille", "Sleep timer"), on ? SleepRules.leftText(d, now, ui.fr) : "");
            List<Runnable> actions = new ArrayList<>();
            for (final int m : SleepRules.CHOICES) actions.add(new Runnable() { public void run() { set(m); } });
            return p.addAll(MenuRules.sleepRows(on ? prefs.getInt("sleep_min", 0) : 0, ui.fr), actions);
        } });
    }

    void set(int minutes) {
        long now = System.currentTimeMillis(), d = SleepRules.deadline(now, minutes);
        if (d == 0) { clear(); ui.toast(SleepRules.offText(ui.fr)); changed.run(); return; }
        prefs.edit().putLong("sleep_at", d).putInt("sleep_min", minutes).apply();
        arm(ctx, d, now);
        ui.toast(KeyService.active == null ? SleepRules.failText(ui.fr) : SleepRules.leftText(d, now, ui.fr));
        changed.run();
    }

    void clear() {
        prefs.edit().remove("sleep_at").remove("sleep_min").apply();
        cancel(ctx);
    }

    // ---------- Alarmes

    static PendingIntent pending(Context c, String action, long at) {
        Intent i = new Intent(action).setClass(c, SleepReceiver.class).putExtra(EXTRA_AT, at);
        int imm = Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0;
        return PendingIntent.getBroadcast(c, 0, i, PendingIntent.FLAG_UPDATE_CURRENT | imm);
    }

    static void arm(Context c, long deadline, long now) {
        AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
        if (am == null) return;
        schedule(am, deadline, pending(c, A_SLEEP, deadline));
        long w = SleepRules.warnAt(deadline, now);
        if (w > 0) schedule(am, w, pending(c, A_WARN, deadline));
        else am.cancel(pending(c, A_WARN, deadline));
    }

    static void cancel(Context c) {
        AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
        if (am == null) return;
        am.cancel(pending(c, A_SLEEP, 0));
        am.cancel(pending(c, A_WARN, 0));
    }

    static void schedule(AlarmManager am, long at, PendingIntent pi) {
        int t = AlarmManager.RTC_WAKEUP;
        if (Build.VERSION.SDK_INT < 23) { am.setExact(t, at, pi); return; }
        try {
            if (Build.VERSION.SDK_INT >= 31 && !am.canScheduleExactAlarms()) am.setAndAllowWhileIdle(t, at, pi);
            else am.setExactAndAllowWhileIdle(t, at, pi);
        } catch (SecurityException e) { am.setAndAllowWhileIdle(t, at, pi); }
    }
}
