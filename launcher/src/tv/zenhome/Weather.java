package tv.zenhome;

import android.content.Context;
import android.content.SharedPreferences;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

import org.json.JSONArray;
import org.json.JSONObject;

/** Meteo (Open-Meteo) : rendu depuis le cache wx_* des preferences, requete reseau, code WMO courant. */
class Weather {
    static final long WX_TTL = 60 * 60_000L;

    final Context ctx;
    final Ui ui;
    final SharedPreferences prefs;
    final Async async;
    TextView tvTemp, tvDesc, tvRange, tvRain;
    LinearLayout box, line, ext, days, hours;   // ext = min/max + 8 heures + 6 jours + mention Open-Meteo, visible quand la ligne a le focus
    // previsions horaires du cache (null = absentes) : heures locales du lieu, probabilite de pluie (%), cumul par quart d'heure (mm)
    long[] hT, qT;
    double[] hTemp, hPop, qMm;
    int[] hCode;
    long[] sunUp, sunDown;   // lever / coucher de chaque jour du cache (heures locales), null si absents
    int utcOff, hoursFirst = -2;
    int descCode;            // ligne du jour : code WMO, texte et picto de repli (le glyphe suit jour / nuit)
    String descCity = "", descPic = "";
    boolean descSet;
    boolean descNight;
    String shown = "";
    boolean busy;
    long tryAt;
    String tryKey = "";

    Weather(Context ctx, Ui ui, SharedPreferences prefs, Async async) {
        this.ctx = ctx; this.ui = ui; this.prefs = prefs; this.async = async;
    }

    void build(ViewGroup parent) {
        // Meteo : une ligne (focalisable, niveau WEATHER) ; au focus, min/max et les 6 jours suivants apparaissent dessous
        // (masquee tant qu'il n'y a ni coordonnees ni donnees)
        box = new LinearLayout(ctx);
        box.setOrientation(LinearLayout.VERTICAL);
        box.setVisibility(View.GONE);
        line = new LinearLayout(ctx);
        line.setOrientation(LinearLayout.HORIZONTAL); line.setGravity(Gravity.BOTTOM);
        line.setPadding(ui.dp(10), ui.dp(2), ui.dp(10), ui.dp(2));
        line.setFocusable(true);
        line.setOnFocusChangeListener(new View.OnFocusChangeListener() { public void onFocusChange(View v, boolean has) {
            v.setBackground(has ? ui.ring(18, 0xF2FFFFFF) : null); } });
        tvTemp = ui.text("", 32, ui.fLight, 0xFFFFFFFF);
        tvDesc = ui.text("", 15, ui.fMed, 0xE6FFFFFF);
        tvDesc.setPadding(ui.dp(10), 0, 0, ui.dp(5));
        tvRain = ui.text("", 11, ui.fMed, 0xFFFFFFFF);   // « Pluie dans l'heure » (masque tant qu'il ne pleut pas)
        tvRain.setShadowLayer(0, 0, 0, 0);
        tvRain.setPadding(ui.dp(10), ui.dp(4), ui.dp(10), ui.dp(4));
        GradientDrawable rb = new GradientDrawable(); rb.setCornerRadius(ui.dp(12)); rb.setColor(0x597CC4FF);
        tvRain.setBackground(rb);
        tvRain.setVisibility(View.GONE);
        line.addView(tvTemp); line.addView(tvDesc);
        LinearLayout.LayoutParams lpR = new LinearLayout.LayoutParams(-2, -2); lpR.leftMargin = ui.dp(12); lpR.bottomMargin = ui.dp(4);
        line.addView(tvRain, lpR);
        tvRange = ui.text("", 11, ui.fReg, 0xB3FFFFFF);
        hours = new LinearLayout(ctx);
        hours.setOrientation(LinearLayout.HORIZONTAL);
        days = new LinearLayout(ctx);
        days.setOrientation(LinearLayout.HORIZONTAL);
        ext = new LinearLayout(ctx);
        ext.setOrientation(LinearLayout.VERTICAL);
        ext.setPadding(ui.dp(10), 0, 0, 0);
        ext.setAlpha(0f);
        ext.addView(tvRange);
        LinearLayout.LayoutParams lpWd = new LinearLayout.LayoutParams(-2, -2); lpWd.topMargin = ui.dp(4);
        ext.addView(hours, lpWd);
        ext.addView(days, lpWd);
        TextView credit = ui.text(Attribution.weather(ui.fr), 10, ui.fReg, 0xFFFFFFFF);   // attribution Open-Meteo, discrete
        credit.setAlpha(0.5f); credit.setShadowLayer(0, 0, 0, 0);
        LinearLayout.LayoutParams lpCr = new LinearLayout.LayoutParams(-2, -2); lpCr.topMargin = ui.dp(6);
        ext.addView(credit, lpCr);
        box.addView(line); box.addView(ext);
        FrameLayout.LayoutParams lpW = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.START);
        lpW.setMargins(ui.dp(46), ui.dp(72), 0, 0);   // 46 + 10 de marge interne : texte aligne sur la salutation (56)
        parent.addView(box, lpW);
    }

    /** Meteo du cache pour l'ecran de veille. */
    static class Detail {
        double temp = Double.NaN, min = Double.NaN, max = Double.NaN, tomorrowMax = Double.NaN;
        int code = 3, tomorrowCode = 3;
        boolean night;
    }

    /**
     * Donnees meteo de l'ecran de veille depuis le cache (3 h au plus), sans requete ; null si absentes ou trop vieilles.
     * today : yyyy-MM-dd ; sans ce jour dans le cache, pas de min / max ni de demain.
     */
    static Detail detail(SharedPreferences prefs, String today) {
        try {
            if (ConfigRules.coords(prefs.getString("lat", ""), prefs.getString("lon", "")) == null) return null;
            long ts = prefs.getLong("wx_ts", 0), now = System.currentTimeMillis();
            if (!StandbyRules.fresh(ts, now)) return null;
            JSONObject o = new JSONObject(prefs.getString("wx_json", ""));
            JSONObject cur = o.getJSONObject("current"), dl = o.getJSONObject("daily");
            JSONArray tm = dl.getJSONArray("time"), wc = dl.getJSONArray("weather_code"), mx = dl.getJSONArray("temperature_2m_max"), mn = dl.getJSONArray("temperature_2m_min");
            Detail r = new Detail();
            r.temp = cur.getDouble("temperature_2m"); r.code = cur.getInt("weather_code");
            int i = -1;
            for (int k = 0; k < tm.length(); k++) if (today.equals(tm.optString(k))) i = k;
            if (i >= 0) {
                r.min = mn.optDouble(i, Double.NaN); r.max = mx.optDouble(i, Double.NaN);
                if (i + 1 < tm.length()) { r.tomorrowMax = mx.optDouble(i + 1, Double.NaN); r.tomorrowCode = wc.optInt(i + 1, 3); }
            }
            int off = o.optInt("utc_offset_seconds", java.util.TimeZone.getDefault().getOffset(now) / 1000);
            r.night = WeatherRules.isNight(WeatherRules.nowLocal(now, off), times(dl.optJSONArray("sunrise")), times(dl.optJSONArray("sunset")));
            return r;
        } catch (Throwable e) { return null; }
    }

    /** La ligne du jour est affichee (donc focalisable). */
    boolean available() { return box.getVisibility() == View.VISIBLE; }

    /** Fondu de 200 ms du bloc (efface quand la mise en avant d'un film prend sa place). */
    void fade(float alpha) { box.animate().alpha(alpha).setDuration(200).start(); }

    /** Coordonnees valides lues dans les prefs, ou null. */
    double[] coords() { return ConfigRules.coords(prefs.getString("lat", ""), prefs.getString("lon", "")); }

    /** Code WMO courant du cache meteo, ou -1 s'il n'y en a pas. */
    int currentCode() { return code(prefs); }

    /** Code WMO courant du cache (sans instance : ecran de veille), ou -1. */
    static int code(SharedPreferences prefs) {
        if (ConfigRules.coords(prefs.getString("lat", ""), prefs.getString("lon", "")) == null) return -1;
        try { return new JSONObject(prefs.getString("wx_json", "")).getJSONObject("current").getInt("weather_code"); }
        catch (Throwable e) { return -1; }
    }

    /** Relance la requete si le cache est trop vieux (appele par dataTick, au premier plan). */
    void refresh(long now) {
        double[] c = coords();
        if (c != null && !busy && Fmt.stale(prefs.getLong("wx_ts", 0), now, WX_TTL)) {
            String key = c[0] + "," + c[1];
            if (!key.equals(tryKey) || now - tryAt > Async.RETRY_MS) { tryKey = key; tryAt = now; fetch(c[0], c[1]); }
        }
    }

    String dayName(String ymd) {
        try {
            Date d = new SimpleDateFormat("yyyy-MM-dd", Locale.US).parse(ymd);
            String s = new SimpleDateFormat("EEE", Locale.getDefault()).format(d).replace(".", "");
            return s.substring(0, 1).toUpperCase(Locale.getDefault()) + s.substring(1);
        } catch (Exception e) { return ""; }
    }

    /** Affiche la meteo depuis le cache des prefs ; masque le bloc s'il n'y a rien d'exploitable. */
    void render() {
        String js = coords() != null ? prefs.getString("wx_json", null) : null;
        String city = prefs.getString("city", "");
        String sig = js == null ? "" : js + "|" + city;
        if (sig.equals(shown)) { tick(); return; }   // donnees inchangees : l'heure et la pluie, elles, avancent
        shown = sig;
        boolean ok = false;
        if (js != null) {
            try {
                JSONObject o = new JSONObject(js);
                JSONObject cur = o.getJSONObject("current"), dl = o.getJSONObject("daily");
                JSONArray tm = dl.getJSONArray("time"), wc = dl.getJSONArray("weather_code");
                JSONArray mx = dl.getJSONArray("temperature_2m_max"), mn = dl.getJSONArray("temperature_2m_min");
                String[] now = Wmo.of(cur.getInt("weather_code"), ui.fr);
                parseHourly(o);   // heures, lever / coucher : avant les icones jour / nuit
                String temp = Fmt.deg(cur.getDouble("temperature_2m"));
                String range = ui.t("Min ", "Low ") + Fmt.deg(mn.getDouble(0)) + "  ·  " + ui.t("Max ", "High ") + Fmt.deg(mx.getDouble(0));
                days.removeAllViews();
                for (int i = 1; i < 7 && i < tm.length(); i++) {
                    LinearLayout cell = new LinearLayout(ctx);
                    cell.setOrientation(LinearLayout.VERTICAL); cell.setGravity(Gravity.CENTER_HORIZONTAL);
                    cell.addView(ui.text(dayName(tm.getString(i)), 10, ui.fMed, 0xCCFFFFFF));
                    cell.addView(ui.icon(IconRules.weatherIcon(wc.getInt(i), false), Wmo.of(wc.getInt(i), ui.fr)[0], 20, 0xFFFFFFFF));
                    cell.addView(ui.text(Math.round(mx.getDouble(i)) + "°/" + Math.round(mn.getDouble(i)) + "°", 10, ui.fReg, 0xE6FFFFFF));
                    days.addView(cell, new LinearLayout.LayoutParams(ui.dp(68), -2));
                }
                tvTemp.setText(temp);
                descCode = cur.getInt("weather_code"); descPic = now[0]; descCity = city.isEmpty() ? "" : "  ·  " + city; descSet = true;
                applyDesc(WeatherRules.isNight(WeatherRules.nowLocal(System.currentTimeMillis(), utcOff), sunUp, sunDown));
                tvRange.setText(range);
                ok = true;
            } catch (Throwable ignored) { }
        }
        box.setVisibility(ok ? View.VISIBLE : View.GONE);
        if (!ok) { hT = qT = null; }
        hoursFirst = -2;
        tick();
    }

    /** Ligne du jour : glyphe jour / nuit puis description. */
    void applyDesc(boolean night) {
        descNight = night;
        tvDesc.setText(ui.picText(Ui.PIC + " " + Wmo.label(descCode, ui.fr, night) + descCity, IconRules.weatherIcon(descCode, night), descPic, 1.25f));
    }

    /** Heures locales d'un tableau Open-Meteo d'horodatages (-1 pour une entree illisible), null s'il manque. */
    static long[] times(JSONArray a) {
        if (a == null) return null;
        long[] r = new long[a.length()];
        for (int i = 0; i < r.length; i++) r[i] = WeatherRules.local(a.optString(i, ""));
        return r;
    }

    /** Lit hourly et minutely_15 du cache ; absents ou illisibles (ancien cache) : pas de rangee d'heures ni de pluie. */
    void parseHourly(JSONObject o) {
        hT = qT = null; sunUp = sunDown = null;
        utcOff = o.optInt("utc_offset_seconds", java.util.TimeZone.getDefault().getOffset(System.currentTimeMillis()) / 1000);
        try {
            JSONObject dl = o.getJSONObject("daily");
            sunUp = times(dl.optJSONArray("sunrise")); sunDown = times(dl.optJSONArray("sunset"));
        } catch (Throwable ignored) { }
        try {
            JSONObject h = o.getJSONObject("hourly");
            JSONArray t = h.getJSONArray("time"), tp = h.getJSONArray("temperature_2m"), pp = h.getJSONArray("precipitation_probability"), wc = h.getJSONArray("weather_code");
            int n = t.length();
            long[] ts = new long[n]; double[] temp = new double[n], pop = new double[n]; int[] code = new int[n];
            for (int i = 0; i < n; i++) {
                ts[i] = WeatherRules.local(t.optString(i, ""));
                temp[i] = tp.optDouble(i, Double.NaN); pop[i] = pp.optDouble(i, -1); code[i] = wc.optInt(i, 3);
            }
            hT = ts; hTemp = temp; hPop = pop; hCode = code;
        } catch (Throwable ignored) { }
        try {
            JSONObject m = o.getJSONObject("minutely_15");
            JSONArray t = m.getJSONArray("time"), pr = m.getJSONArray("precipitation");
            int n = t.length();
            long[] ts = new long[n]; double[] mm = new double[n];
            for (int i = 0; i < n; i++) { ts[i] = WeatherRules.local(t.optString(i, "")); mm[i] = pr.optDouble(i, 0); }
            qT = ts; qMm = mm;
        } catch (Throwable ignored) { }
    }

    /** Rangee des 8 prochaines heures (reconstruite quand l'heure change) et badge de pluie. */
    void tick() {
        long now = WeatherRules.nowLocal(System.currentTimeMillis(), utcOff);
        int[] ix = hT == null ? new int[0] : WeatherRules.nextHours(hT, now, 8);
        int first = ix.length == 0 ? -1 : ix[0];
        if (first != hoursFirst) {
            hoursFirst = first;
            hours.removeAllViews();
            for (int i : ix) {
                LinearLayout cell = new LinearLayout(ctx);
                cell.setOrientation(LinearLayout.VERTICAL); cell.setGravity(Gravity.CENTER_HORIZONTAL);
                cell.addView(ui.text(WeatherRules.hourLabel(hT[i], ui.fr), 10, ui.fMed, 0xCCFFFFFF));
                cell.addView(ui.icon(IconRules.weatherIcon(hCode[i], WeatherRules.isNight(hT[i], sunUp, sunDown)), Wmo.of(hCode[i], ui.fr)[0], 20, 0xFFFFFFFF));
                cell.addView(ui.text(Double.isNaN(hTemp[i]) ? "–" : Fmt.deg(hTemp[i]), 10, ui.fReg, 0xE6FFFFFF));
                String pop = WeatherRules.popText(hPop[i]);
                TextView pt = ui.text("", 10, ui.fReg, Ui.ACCENT);
                pt.setText(pop.isEmpty() ? "" : ui.picText(Ui.PIC + pop, "drop", "", 1.1f));   // goutte + pourcentage
                cell.addView(pt);
                hours.addView(cell, new LinearLayout.LayoutParams(ui.dp(51), -2));
            }
            hours.setVisibility(ix.length == 0 ? View.GONE : View.VISIBLE);
        }
        if (descSet && WeatherRules.isNight(now, sunUp, sunDown) != descNight) applyDesc(!descNight);   // le jour tombe
        String rain = WeatherRules.rainText(WeatherRules.rainStart(hT, hPop, qT, qMm, now), now, ui.fr);
        tvRain.setText(rain);
        tvRain.setVisibility(rain.isEmpty() ? View.GONE : View.VISIBLE);
    }

    void fetch(final double lat, final double lon) {
        busy = true;
        final String url = String.format(Locale.US, "https://api.open-meteo.com/v1/forecast?latitude=%.4f&longitude=%.4f"
                + "&current=temperature_2m,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min,sunrise,sunset"
                + "&hourly=temperature_2m,precipitation_probability,weather_code&minutely_15=precipitation&forecast_minutely_15=8"
                + "&timezone=auto&forecast_days=7", lat, lon);
        async.bg(new Runnable() { public void run() {
            try {
                if (async.fg) {
                    String js = Net.json(url, null);
                    JSONObject o = new JSONObject(js);   // doit etre exploitable avant de remplacer le cache
                    o.getJSONObject("current").getInt("weather_code");
                    o.getJSONObject("daily").getJSONArray("time");
                    double[] cur = coords();   // coordonnees changees pendant la requete : on abandonne
                    if (cur == null || Math.abs(cur[0] - lat) > 1e-4 || Math.abs(cur[1] - lon) > 1e-4) return;
                    prefs.edit().putString("wx_json", js).putLong("wx_ts", System.currentTimeMillis()).apply();
                }
            } catch (Throwable ignored) { }
            finally { async.ui.post(new Runnable() { public void run() { busy = false; if (!async.destroyed) render(); } }); }
        } });
    }
}
