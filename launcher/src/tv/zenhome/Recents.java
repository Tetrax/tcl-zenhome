package tv.zenhome;

import android.app.ActivityManager;
import android.app.AppOpsManager;
import android.app.usage.UsageEvents;
import android.app.usage.UsageStatsManager;
import android.content.Context;
import android.os.Build;

import java.io.BufferedReader;
import java.io.FileReader;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Applis ouvertes : menu lateral des applis utilisees recemment (probablement encore en memoire), fermeture d'une appli ou de toutes
 * (killBackgroundProcesses : ne touche que les processus d'arriere-plan), gain mesure sur MemAvailable. Les stats d'usage demandent
 * PACKAGE_USAGE_STATS (accordee par adb : appops set tv.zenhome GET_USAGE_STATS allow) ; sinon seul « Tout liberer » reste, sur le dock
 * et le carrousel. Zen Home et les paquets systeme critiques ne sont jamais proposes (RecentRules).
 */
class Recents {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final Overlays ovs;
    final HomeConfig config;
    final Carousel carousel;
    final Set<String> closed = new HashSet<>();   // fermees depuis l'ouverture du menu : retirees de la liste
    String status = "";                           // sous-titre apres « Tout liberer » (gain mesure)
    MenuPanel panel;

    Recents(Context ctx, Ui ui, Apps apps, Overlays ovs, HomeConfig config, Carousel carousel) {
        this.ctx = ctx; this.ui = ui; this.apps = apps; this.ovs = ovs; this.config = config; this.carousel = carousel;
    }

    boolean usageAccess() {
        try {
            AppOpsManager ops = (AppOpsManager) ctx.getSystemService(Context.APP_OPS_SERVICE);
            return ops != null && ops.checkOpNoThrow("android:get_usage_stats", android.os.Process.myUid(), ctx.getPackageName()) == AppOpsManager.MODE_ALLOWED;
        } catch (Throwable e) { return false; }
    }

    /** Dernier usage (ms) de chaque paquet vu passer au premier plan ou en pause dans les 3 dernieres heures. */
    Map<String, Long> lastUsed(long now) {
        Map<String, Long> m = new HashMap<>();
        if (Build.VERSION.SDK_INT < 22) return m;
        try {
            UsageStatsManager usm = (UsageStatsManager) ctx.getSystemService(Context.USAGE_STATS_SERVICE);
            UsageEvents ev = usm.queryEvents(now - RecentRules.WINDOW_MS, now);
            UsageEvents.Event e = new UsageEvents.Event();
            while (ev != null && ev.hasNextEvent()) {
                ev.getNextEvent(e);
                if (e.getEventType() != 1 && e.getEventType() != 2) continue;   // ACTIVITY_RESUMED / ACTIVITY_PAUSED
                Long old = m.get(e.getPackageName());
                if (old == null || old < e.getTimeStamp()) m.put(e.getPackageName(), e.getTimeStamp());
            }
        } catch (Throwable ignored) { }
        return m;
    }

    /** MemAvailable en ko, -1 si illisible. */
    static long availKb() {
        BufferedReader br = null;
        try {
            br = new BufferedReader(new FileReader("/proc/meminfo"));
            String l;
            while ((l = br.readLine()) != null) if (l.startsWith("MemAvailable:")) return Fmt.kb(l);
        } catch (Exception ignored) { }
        finally { try { if (br != null) br.close(); } catch (Exception ignored) { } }
        return -1;
    }

    void kill(String pkg) {
        if (!RecentRules.canClose(pkg)) return;
        try { ((ActivityManager) ctx.getSystemService(Context.ACTIVITY_SERVICE)).killBackgroundProcesses(pkg); } catch (Throwable ignored) { }
    }

    /** « Fermer l'appli » d'un menu de tuile : ferme et dit ce qu'on sait (recente ou non ; inconnu sans acces aux stats). */
    void closeApp(String pkg) {
        if (!RecentRules.canClose(pkg)) return;
        Boolean recent = null;
        if (usageAccess()) recent = RecentRules.select(lastUsed(System.currentTimeMillis()), System.currentTimeMillis(), RecentRules.WINDOW_MS, 1000).contains(pkg);
        kill(pkg);
        ui.toast(RecentRules.closedText(apps.appLabel(pkg), recent, ui.fr));
    }

    void show() {
        closed.clear(); status = "";
        panel = new MenuPanel(ctx, ui, ovs);
        panel.show(new MenuPanel.Source() { public MenuPanel.Page build() { return page(); } });
    }

    /** Applis a proposer : les recentes (avec acces aux stats), sinon le dock et le carrousel ; lancables, non fermees ici. */
    MenuPanel.Page page() {
        final long now = System.currentTimeMillis();
        boolean access = usageAccess();
        Map<String, Long> last = access ? lastUsed(now) : new HashMap<String, Long>();
        List<String> src = access ? RecentRules.select(last, now, RecentRules.WINDOW_MS, 3 * RecentRules.MAX) : new ArrayList<String>();
        if (!access) { src.addAll(config.dockPkgs); src.addAll(carousel.pkgs); }
        final List<String> pkgs = new ArrayList<>();
        for (String p : src) {
            if (pkgs.size() >= RecentRules.MAX) break;
            if (!pkgs.contains(p) && RecentRules.canClose(p) && !closed.contains(p) && apps.usable(p)) pkgs.add(p);
        }
        String ram = RecentRules.ramText(availKb(), ui.fr);
        MenuPanel.Page p = new MenuPanel.Page(ui.t("Applis ouvertes", "Open apps"), status.isEmpty() ? ram : status);
        p.addStay(MenuRules.action(ui.t("Tout libérer", "Free all"), "broom"), new Runnable() { public void run() { freeAll(pkgs); } });
        if (!access) p.add(MenuRules.info(ui.t("Autorisation requise : voir README", "Permission required: see README")), null);
        else if (pkgs.isEmpty()) p.add(MenuRules.info(ui.t("Aucune appli récente", "No recent apps")), null);
        else for (final String pkg : pkgs)
            p.addApp(MenuRules.app(apps.appLabel(pkg), RecentRules.agoText(now, last.get(pkg), ui.fr)), apps.pkgIcon(pkg),
                    new Runnable() { public void run() { closed.add(pkg); kill(pkg); } });
        return p;
    }

    /** Ferme toutes les applis listees, puis mesure le gain de MemAvailable apres ~800 ms. */
    void freeAll(List<String> pkgs) {
        final long before = availKb();
        final int n = pkgs.size();
        for (String p : pkgs) { closed.add(p); kill(p); }
        status = "";
        panel.pages.postDelayed(new Runnable() { public void run() {
            status = RecentRules.freeAllText(n, before, availKb(), ui.fr);
            if (panel.alive()) panel.refresh(0);
        } }, 800);
    }
}
