package tv.zenhome;

import android.content.Context;
import android.content.SharedPreferences;
import android.graphics.Typeface;
import android.text.TextUtils;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.text.SimpleDateFormat;
import java.util.Calendar;
import java.util.Date;
import java.util.Locale;

/**
 * Lignes discretes sous l'heure et la date de l'ecran de veille (ambiance et economiseur) : meteo, demain, prochaine sortie
 * de Ma liste, minuteur, phrase du jour. Donnees des caches existants, aucun reseau ; une ligne sans donnee est masquee ;
 * mise a jour a la minute seulement.
 */
class Standby {
    final Ui ui;
    final SharedPreferences prefs;
    final LinearLayout box;
    final TextView wx, tom, rel, slp, phr;
    long lastMin = -1;

    Standby(Context ctx, Ui ui, SharedPreferences prefs, float size) {
        this.ui = ui; this.prefs = prefs;
        box = new LinearLayout(ctx);
        box.setOrientation(LinearLayout.VERTICAL);
        box.setAlpha(0.75f);
        box.setPadding(0, ui.dp(14), 0, 0);
        wx = line(size, ui.fReg); tom = line(size, ui.fReg); rel = line(size, ui.fReg); slp = line(size, ui.fReg);
        phr = line(size, Typeface.create(ui.fLight, Typeface.ITALIC));   // italique (synthetise si la police n'en a pas)
    }

    TextView line(float size, Typeface tf) {
        TextView t = ui.text("", size, tf, 0xFFFFFFFF);
        t.setSingleLine(true); t.setEllipsize(TextUtils.TruncateAt.END); t.setMaxWidth(ui.dp(560));
        t.setVisibility(View.GONE);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-2, -2); lp.topMargin = ui.dp(4);
        box.addView(t, lp);
        return t;
    }

    static void set(TextView t, CharSequence s) {
        t.setText(s);
        t.setVisibility(s.length() == 0 ? View.GONE : View.VISIBLE);
    }

    /** Relit les caches ; sans force, seulement quand la minute a change. */
    void update(boolean force) {
        long now = System.currentTimeMillis(), min = now / 60_000L;
        if (!force && min == lastMin) return;
        lastMin = min;
        String today = new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date(now));
        Weather.Detail w = Weather.detail(prefs, today);
        if (w == null) { set(wx, ""); set(tom, ""); }
        else {
            set(wx, ui.picText(StandbyRules.weatherLine(w.temp, w.code, w.min, w.max, ui.fr, Ui.PIC, w.night), IconRules.weatherIcon(w.code, w.night), Wmo.of(w.code, ui.fr)[0], 1.2f));
            set(tom, ui.picText(StandbyRules.tomorrowLine(w.tomorrowMax, w.tomorrowCode, ui.fr, Ui.PIC), IconRules.weatherIcon(w.tomorrowCode, false), Wmo.of(w.tomorrowCode, ui.fr)[0], 1.2f));
        }
        Movie m = StandbyRules.nextRelease(Movies.parseMovies(prefs.getString(MovieRules.jsonKey(3), null)), today);
        set(rel, m == null ? "" : StandbyRules.releaseText(m.title, m.date, today, ui.fr));
        set(slp, SleepRules.leftText(prefs.getLong("sleep_at", 0), now, ui.fr));
        Calendar c = Calendar.getInstance();
        int slot = Greeting.slotOf(c.get(Calendar.HOUR_OF_DAY));
        set(phr, Greeting.phrase(c.get(Calendar.DAY_OF_YEAR), c.get(Calendar.DAY_OF_WEEK), slot, Weather.code(prefs), prefs.getString("city", ""), ui.fr));
    }
}
