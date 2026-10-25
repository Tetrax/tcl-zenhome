package tv.zenhome;

import android.content.Context;
import android.content.SharedPreferences;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.TextView;

import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.List;
import java.util.Locale;

/**
 * Bandeau « <film> sort aujourd'hui » pour les films de Ma liste : une fois par jour (prefs list_banner_day), 10 s puis fondu.
 * Il prend la place de la ligne meteo pendant ce temps. Aucun reseau : Ma liste est locale.
 */
class Banner {
    static final long SHOW_MS = 10_000L;

    final Context ctx;
    final Ui ui;
    final SharedPreferences prefs;
    final Async async;
    final Weather weather;
    final Host host;
    TextView tv;

    Banner(Context ctx, Ui ui, SharedPreferences prefs, Async async, Weather weather, Host host) {
        this.ctx = ctx; this.ui = ui; this.prefs = prefs; this.async = async; this.weather = weather; this.host = host;
    }

    void build(ViewGroup parent) {
        tv = ui.text("", 13, ui.fMed, 0xFFFFFFFF);
        tv.setShadowLayer(0, 0, 0, 0);
        tv.setSingleLine(true);
        tv.setEllipsize(android.text.TextUtils.TruncateAt.END);
        tv.setMaxWidth(ui.dp(700));
        tv.setPadding(ui.dp(12), ui.dp(5), ui.dp(12), ui.dp(5));
        GradientDrawable bg = new GradientDrawable(); bg.setCornerRadius(ui.dp(14)); ui.theme.bind(bg, Theme.PILL, 0);
        tv.setBackground(bg);
        tv.setVisibility(View.GONE);
        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.START);
        lp.setMargins(ui.dp(46), ui.dp(76), 0, 0);   // sous la salutation, a la place de la ligne meteo
        parent.addView(tv, lp);
    }

    /** onResume (apres le vidage de async.h) : montre le bandeau s'il y a une sortie aujourd'hui et qu'il n'a pas encore ete vu. */
    void check(List<Movie> myList) {
        String today = new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date());
        List<String> titles = MovieRules.releasingToday(myList, today);
        if (!MovieRules.bannerDue(prefs.getString("list_banner_day", ""), today, titles.size())) return;
        prefs.edit().putString("list_banner_day", today).apply();
        tv.setText(MovieRules.bannerText(titles, ui.fr));
        tv.animate().cancel();
        tv.setAlpha(0f); tv.setVisibility(View.VISIBLE);
        tv.animate().alpha(1f).setDuration(400).start();
        weather.fade(0f);
        async.h.postDelayed(hide, SHOW_MS);
    }

    final Runnable hide = new Runnable() { public void run() { stop(true); } };

    /** Fin du bandeau (10 s ecoulees ou onPause) ; la meteo revient, sauf si la mise en avant d'un film l'a deja remplacee. */
    void stop(boolean anim) {
        async.h.removeCallbacks(hide);
        if (tv.getVisibility() != View.VISIBLE) return;
        tv.animate().cancel();
        if (anim) tv.animate().alpha(0f).setDuration(400).withEndAction(new Runnable() { public void run() { tv.setVisibility(View.GONE); } }).start();
        else tv.setVisibility(View.GONE);
        if (host.level() != Nav.State.FILMS) weather.fade(1f);
    }
}
