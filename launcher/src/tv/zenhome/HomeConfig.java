package tv.zenhome;

import android.content.Context;
import android.content.SharedPreferences;
import android.provider.Settings;
import android.text.TextUtils;
import android.util.Base64;

import java.util.ArrayList;
import java.util.List;

/**
 * Configuration : preferences de la TV, remplacees par celles du kit (Settings.Global zenhome_*) si plus recentes.
 * Le fichier de preferences « zenhome » et ses cles sont un contrat avec ConfigReceiver.
 */
class HomeConfig {
    /** Applis de streaming connues, par ordre de priorite pour le dock automatique. */
    static final String[] KNOWN_DOCK = {
            "com.netflix.ninja", "com.google.android.youtube.tv", "com.amazon.amazonvideo.livingroom",
            "com.disney.disneyplus", "com.canal.android.canal", "com.wbd.stream",
            "com.apple.atve.androidtv.appletv", "com.spotify.tv.android", "com.plexapp.android",
            "fr.francetv.pluzz", "fr.tf1.mytf1", "fr.m6.m6replay", "tv.arte.plus7", "tv.molotov.app",
            "com.cbs.ott", "com.dazn", "tv.twitch.android.app", "org.xbmc.kodi", "com.google.android.youtube.tvkids",
    };

    final Context ctx;
    final SharedPreferences prefs;
    final Apps apps;
    final Host host;

    long rotationMs = 60_000L;
    List<String> dockPkgs = new ArrayList<>();
    int layout = 4;               // nombre d'applis du dock, 2 a 10 (l'ancienne pref 4/6/8 reste valable)
    String titleMode = "device";  // device | none | custom
    String titleText = "";

    HomeConfig(Context ctx, SharedPreferences prefs, Apps apps, Host host) {
        this.ctx = ctx; this.prefs = prefs; this.apps = apps; this.host = host;
    }

    String setting(String key) {
        try { return Settings.Global.getString(ctx.getContentResolver(), key); } catch (Throwable e) { return null; }
    }

    /** Intervalle entre deux photos (zenhome_interval). */
    void loadInterval() {
        try {
            String iv = setting("zenhome_interval");
            if (iv != null) rotationMs = Math.max(10, Integer.parseInt(iv.trim())) * 1000L;
        } catch (Exception ignored) { }
    }

    /** Un nouveau zenhome_rev du kit n'a pas encore ete applique. */
    boolean revChanged() {
        String rev = setting("zenhome_rev");
        return rev != null && !rev.equals(prefs.getString("rev", null));
    }

    void load() {
        String rev = setting("zenhome_rev");
        if (rev != null && !rev.equals(prefs.getString("rev", null))) {
            SharedPreferences.Editor e = prefs.edit().putString("rev", rev);
            String mode = setting("zenhome_title_mode");
            if (mode != null) e.putString("title_mode", mode.trim());
            String b64 = setting("zenhome_title_b64");
            if (b64 != null && b64.trim().length() > 0) {
                try { e.putString("title", new String(Base64.decode(b64.trim(), Base64.DEFAULT), "UTF-8").trim()); } catch (Exception ignored) { }
            }
            String th = ThemeRules.fromSetting(setting("zenhome_theme"));
            if (th != null) e.putString("theme", th);
            String lay = setting("zenhome_layout");
            if (lay != null) e.putString("layout", lay.trim());
            String d = setting("zenhome_dock");
            if (d != null) {
                // "auto" : le kit demande le choix automatique des applis
                if ("auto".equalsIgnoreCase(d.trim())) e.putString("dock", "").putBoolean("dock_user_set", false);
                else e.putString("dock", d.trim()).putBoolean("dock_user_set", true);
            }
            e.apply();
        }
        titleMode = prefs.getString("title_mode", "device");
        titleText = prefs.getString("title", "");
        layout = ConfigRules.layout(prefs.getString("layout", "4"));
        dockPkgs.clear();
        String csv = prefs.getString("dock", "");
        if (csv.length() > 0) {
            for (String p : ConfigRules.csv(csv)) { if (apps.usable(p) && !dockPkgs.contains(p)) dockPkgs.add(p); }
        }
        if (dockPkgs.isEmpty() && !prefs.getBoolean("dock_user_set", false)) {
            for (String p : KNOWN_DOCK) { if (dockPkgs.size() >= layout) break; if (apps.usable(p)) dockPkgs.add(p); }
        }
        while (dockPkgs.size() > layout) dockPkgs.remove(dockPkgs.size() - 1);
    }

    void save() {
        prefs.edit()
                .putString("dock", TextUtils.join(",", dockPkgs))
                .putBoolean("dock_user_set", true)
                .putString("layout", String.valueOf(layout))
                .putString("title_mode", titleMode)
                .putString("title", titleText)
                .apply();
        if (!prefs.getBoolean("carousel_user_set", false)) host.carouselRefresh();   // le dock a change
    }
}
