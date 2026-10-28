package tv.zenhome;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;

import java.io.File;
import java.util.Map;

/**
 * Configuration privee envoyee par adb (protegee par la permission DUMP : seul le shell/systeme peut l'appeler).
 * Ecrit dans les preferences privees "zenhome" (lues par Home). Ne journalise JAMAIS le jeton.
 *
 *   adb shell am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --es lat 48.86 --es lon 2.35 --es city Paris
 *   adb shell am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --es tmdb_token "$(cat tmdb.token)"
 *   adb shell am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --ez clear_tmdb true
 *
 * (jeton lu dans un fichier local : jamais en clair dans la commande ni dans l'historique du shell)
 *   adb shell am broadcast -a tv.zenhome.CONFIG -n tv.zenhome/.ConfigReceiver --ez labels true
 *
 * Extras (tous optionnels) : tmdb_token, lat, lon, city (chaines), clear_tmdb, labels (booleens).
 * labels : renvoie (setResultData, code 1) une ligne « paquet<TAB>libelle » par appli lancable ; rien d'autre n'est modifie.
 * Valeurs invalides ignorees sans bruit.
 */
public class ConfigReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context c, Intent i) {
        if (i == null || !"tv.zenhome.CONFIG".equals(i.getAction())) return;
        if (i.getBooleanExtra("labels", false)) { labels(c); return; }   // lecture seule : aucune pref touchee
        SharedPreferences.Editor e = c.getSharedPreferences("zenhome", Context.MODE_PRIVATE).edit();
        boolean posters = false;

        if (i.getBooleanExtra("clear_tmdb", false)) {
            e.remove("tmdb_token");
            clearMovies(e);
            posters = true;
        }
        String tok = ConfigRules.token(i.getStringExtra("tmdb_token"));   // jamais de saut de ligne dans un en-tete
        if (tok != null) {
            e.putString("tmdb_token", tok);
            clearMovies(e);
        }

        boolean geo = false;
        String lat = i.getStringExtra("lat"), lon = i.getStringExtra("lon");
        if (ConfigRules.valid(lat, 90)) { e.putString("lat", lat.trim()); geo = true; }
        if (ConfigRules.valid(lon, 180)) { e.putString("lon", lon.trim()); geo = true; }
        String city = i.getStringExtra("city");
        if (city != null) {
            city = ConfigRules.city(city);
            if (city.isEmpty()) e.remove("city"); else e.putString("city", city);
            geo = true;
        }
        if (geo) e.remove("wx_json").remove("wx_ts");
        e.commit();

        if (posters) {
            for (String d : new String[]{"posters", "backdrops"}) {
                File[] fs = new File(c.getCacheDir(), d).listFiles();
                if (fs != null) for (File f : fs) f.delete();
            }
        }
    }

    void labels(Context c) {
        PackageManager pm = c.getPackageManager();
        StringBuilder sb = new StringBuilder();
        for (Map.Entry<String, ResolveInfo> en : Apps.launchable(c).entrySet()) {
            String line = ConfigRules.labelLine(en.getKey(), en.getValue().loadLabel(pm));
            if (line != null) sb.append(sb.length() > 0 ? "\n" : "").append(line);
        }
        setResultCode(1);
        setResultData(sb.toString());
    }

    /**
     * Caches des onglets distants 0..2 (Netflix, cinema, bientot). L'onglet 3 (« Ma liste », prefs my_list) est volontairement
     * exclu de la boucle : il est local et doit survivre a clear_tmdb.
     */
    static void clearMovies(SharedPreferences.Editor e) {
        for (int k = 0; k < 3; k++) e.remove(MovieRules.jsonKey(k)).remove(MovieRules.tsKey(k));
    }
}
