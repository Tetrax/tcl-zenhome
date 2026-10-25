package tv.zenhome;

import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Detail d'un film en surimpression : fiche, bouton, image de fond chargee en arriere-plan. */
class MovieDetail {
    final Context ctx;
    final Ui ui;
    final Async async;
    final Overlays ovs;
    final Apps apps;
    final Host host;
    final SharedPreferences prefs;
    FrameLayout detailOv;
    ImageView detailImg;
    Movie detailMovie;
    int detailGen = 0;

    MovieDetail(Context ctx, Ui ui, SharedPreferences prefs, Async async, Overlays ovs, Apps apps, Host host) {
        this.host = host; this.ctx = ctx; this.ui = ui; this.prefs = prefs; this.async = async; this.ovs = ovs; this.apps = apps;
    }

    void show(final Movie m, int nxTab) {
        final FrameLayout ov = new FrameLayout(ctx);
        ov.setBackgroundColor(0xFF08090B);
        ov.setClickable(true);
        detailImg = new ImageView(ctx);
        detailImg.setScaleType(ImageView.ScaleType.CENTER_CROP);
        ov.addView(detailImg, new FrameLayout.LayoutParams(-1, -1));
        View scrim = new View(ctx);
        scrim.setBackground(new GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT, new int[]{0xEE000000, 0x99000000}));
        ov.addView(scrim, new FrameLayout.LayoutParams(-1, -1));

        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        TextView title = ui.text(m.title, 28, ui.fMed, 0xFFFFFFFF);
        title.setMaxLines(2); title.setEllipsize(TextUtils.TruncateAt.END);
        col.addView(title);
        String meta = MovieRules.detailMeta(m, nxTab, ui.fr);
        TextView mt = ui.text(meta, 14, ui.fReg, 0xCCFFFFFF);
        mt.setPadding(0, ui.dp(6), 0, ui.dp(14));
        col.addView(mt);
        TextView ow = ui.text(m.overview.isEmpty() ? ui.t("Résumé indisponible", "Summary unavailable") : m.overview, 14, ui.fReg, 0xE6FFFFFF);
        ow.setMaxLines(8); ow.setEllipsize(TextUtils.TruncateAt.END); ow.setLineSpacing(ui.dp(3), 1f);
        col.addView(ow);

        final boolean netflix = nxTab == 0;   // cinema / bientot : simple bouton Fermer
        final LinearLayout btns = new LinearLayout(ctx);
        btns.setOrientation(LinearLayout.HORIZONTAL);
        TextView btn = button(netflix ? ui.t("Ouvrir Netflix", "Open Netflix") : ui.t("Fermer", "Close"));
        btn.setOnClickListener(new View.OnClickListener() { public void onClick(View v) {
            ovs.closeOverlay(ov, true); if (netflix) apps.launch("com.netflix.ninja"); } });
        btns.addView(btn);
        if (!m.poster.isEmpty()) {   // « Ma liste » : bascule ajouter / retirer
            final TextView lb = button(listLabel(m));
            lb.setOnClickListener(new View.OnClickListener() { public void onClick(View v) {
                host.toggleMyList(m); lb.setText(listLabel(m)); } });
            LinearLayout.LayoutParams ll = new LinearLayout.LayoutParams(-2, -2); ll.leftMargin = ui.dp(12);
            btns.addView(lb, ll);
        }
        LinearLayout.LayoutParams lpb = new LinearLayout.LayoutParams(-2, -2); lpb.topMargin = ui.dp(24);
        col.addView(btns, lpb);

        FrameLayout.LayoutParams lpc = new FrameLayout.LayoutParams(ui.dp(620), -2, Gravity.CENTER_VERTICAL | Gravity.START);
        lpc.leftMargin = ui.dp(90);
        ov.addView(col, lpc);
        TextView credit = ui.text(Attribution.tmdb(ui.fr), 10, ui.fReg, 0xFFFFFFFF);   // attribution TMDB, discrete
        credit.setAlpha(0.5f); credit.setShadowLayer(0, 0, 0, 0);
        credit.setSingleLine(true); credit.setEllipsize(TextUtils.TruncateAt.END);
        FrameLayout.LayoutParams lpt = new FrameLayout.LayoutParams(-1, -2, Gravity.BOTTOM | Gravity.START);
        lpt.setMargins(ui.dp(90), 0, ui.dp(90), ui.dp(20));
        ov.addView(credit, lpt);

        detailOv = ov; detailMovie = m;
        ovs.openOverlay(ov, btn);
        loadBackdrop(m);
        loadTrailer(m, btns);
    }

    String listLabel(Movie m) {
        return host.inMyList(m) ? ui.t("Retirer de ma liste", "Remove from my list") : ui.t("Ajouter à ma liste", "Add to my list");
    }

    /** Bouton du detail : verre translucide, blanc au focus. */
    TextView button(String label) {
        TextView btn = ui.text(label, 15, ui.fMed, 0xFFFFFFFF);
        btn.setShadowLayer(0, 0, 0, 0);
        btn.setPadding(ui.dp(24), ui.dp(11), ui.dp(24), ui.dp(11));
        final GradientDrawable bb = new GradientDrawable(); bb.setCornerRadius(ui.dp(18)); bb.setColor(ui.theme.p.glassFill);
        btn.setBackground(bb);
        btn.setFocusable(true); btn.setClickable(true);
        btn.setOnFocusChangeListener(new View.OnFocusChangeListener() { public void onFocusChange(View v, boolean has) {
            bb.setColor(has ? 0xF2FFFFFF : ui.theme.p.glassFill); ((TextView) v).setTextColor(has ? 0xFF111111 : 0xFFFFFFFF); } });
        return btn;
    }

    /**
     * Bande-annonce : videos TMDB (fr-FR, repli en-US), Trailer + YouTube, officielle en priorite. Le bouton n'apparait
     * que s'il y en a une. Une seule requete a la fois : le detail ferme ou rouvert annule la precedente (detailGen).
     */
    void loadTrailer(final Movie m, final LinearLayout btns) {
        final String tk = prefs.getString("tmdb_token", "");
        if (m.id <= 0 || tk.length() == 0) return;
        final int gen = detailGen;
        async.bg(new Runnable() { public void run() {
            String key = "";
            try {
                for (String lang : new String[]{"fr-FR", "en-US"}) {
                    if (!async.fg || gen != detailGen) return;
                    key = MovieRules.pickTrailer(parseVideos(Net.json(MovieRules.videosUrl(m.id, lang), tk)));
                    if (!key.isEmpty()) break;
                }
            } catch (Throwable ignored) { }
            if (key.isEmpty()) return;
            final String k = key;
            async.ui.post(new Runnable() { public void run() {
                if (async.destroyed || gen != detailGen || detailOv == null || btns.getParent() == null) return;
                TextView tb = button(ui.t("Bande-annonce", "Trailer"));
                tb.setOnClickListener(new View.OnClickListener() { public void onClick(View v) { playTrailer(k); } });
                LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-2, -2); lp.leftMargin = ui.dp(12);
                btns.addView(tb, lp);
            } });
        } });
    }

    static java.util.List<MovieRules.Video> parseVideos(String js) throws org.json.JSONException {
        org.json.JSONArray a = new org.json.JSONObject(js).getJSONArray("results");
        java.util.List<MovieRules.Video> l = new java.util.ArrayList<>();
        for (int i = 0; i < a.length(); i++) {
            org.json.JSONObject o = a.getJSONObject(i);
            l.add(new MovieRules.Video(o.optString("key", ""), o.optString("type", ""), o.optString("site", ""), o.optBoolean("official", false)));
        }
        return l;
    }

    /** Lance YouTube TV sur la cle deja validee (jamais de navigateur : sans l'appli, un message). */
    void playTrailer(String key) {
        String k = MovieRules.youtubeKey(key);
        if (k.isEmpty()) return;
        try {
            ctx.startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(MovieRules.watchUrl(k)))
                    .setPackage("com.google.android.youtube.tv").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        } catch (Exception e) { ui.toast(ui.t("YouTube TV n'est pas installé", "YouTube TV is not installed")); }
    }

    void loadBackdrop(Movie m) {
        if (m.backdrop.isEmpty()) return;
        final int gen = ++detailGen;
        final String url = MovieRules.backdropUrl(m.backdrop);
        async.bg(new Runnable() { public void run() {
            Bitmap b = null;
            try {
                if (async.fg) {
                    byte[] d = Net.image(url);
                    BitmapFactory.Options o = new BitmapFactory.Options();
                    o.inJustDecodeBounds = true;
                    BitmapFactory.decodeByteArray(d, 0, d.length, o);
                    if (MovieRules.tooBig(o.outWidth, o.outHeight)) return;
                    o.inSampleSize = MovieRules.sample(o.outWidth, 1280);
                    o.inJustDecodeBounds = false;
                    o.inPreferredConfig = Bitmap.Config.RGB_565;
                    b = BitmapFactory.decodeByteArray(d, 0, d.length, o);
                }
            } catch (Throwable ignored) { }
            if (b == null) return;
            final Bitmap fb = b;
            async.ui.post(new Runnable() { public void run() {
                if (async.destroyed || gen != detailGen || detailImg == null) { fb.recycle(); return; }
                detailImg.setImageBitmap(fb);
                detailImg.setAlpha(0f); detailImg.animate().alpha(1f).setDuration(300).start();
            } });
        } });
    }

    /** Libere l'image de fond du detail sans fermer l'ecran (onStop). */
    void dropBackdrop() {
        detailGen++;
        if (detailImg == null) return;
        detailImg.animate().cancel();
        Drawable d = detailImg.getDrawable(); detailImg.setImageDrawable(null);
        if (d instanceof BitmapDrawable) { Bitmap b = ((BitmapDrawable) d).getBitmap(); if (b != null) b.recycle(); }
    }

    void freeDetail() {
        detailGen++;   // annule la recherche de bande-annonce en cours
        dropBackdrop();
        detailImg = null; detailOv = null; detailMovie = null;
    }

    /** onResume : l'image de fond a pu etre liberee (onStop) pendant que le detail restait ouvert. */
    void reloadBackdrop() {
        if (detailOv != null && detailImg != null && detailImg.getDrawable() == null && detailMovie != null) loadBackdrop(detailMovie);
    }

    /** Appele par Overlays quand un overlay est retire de la pile. */
    void onOverlayClosed(View ov) { if (ov == detailOv) freeDetail(); }
}
