package tv.zenhome;

import android.content.Context;
import android.content.SharedPreferences;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.drawable.BitmapDrawable;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.io.File;
import java.io.FileOutputStream;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;

import org.json.JSONArray;
import org.json.JSONObject;

/**
 * « Nouveau sur Netflix » (TMDB) : onglets, rangee d'affiches (cache disque + generations), mise en avant du
 * film qui a le focus, requetes et lecture JSON.
 */
class Movies {
    static final long NX_TTL = 6 * 60 * 60_000L;

    final Context ctx;
    final Ui ui;
    final SharedPreferences prefs;
    final Async async;
    final Overlays ovs;
    final Host host;
    LinearLayout nxBox, nxRow, nxTabsRow;
    TextView[] nxTabs = new TextView[MovieRules.TABS];   // onglets : 0 Netflix, 1 Au cinema, 2 Bientot, 3 Ma liste
    int nxTab = 0;
    // mise en avant du film qui a le focus (remplace la meteo)
    LinearLayout spBox;
    TextView spTitle, spMeta, spSum;
    boolean spOn = false;
    Movie spMovie;
    boolean tabNav = false;   // vrai pendant une fleche ◀ ▶ de l'utilisateur sur une pastille
    String nxShown = "";
    boolean nxBusy, posterBusy;
    long nxTry;
    String nxTryKey = "";
    int posterGen = 0;
    List<Movie> movies = new ArrayList<>();
    List<ImageView> posterIv = new ArrayList<>();

    Movies(Context ctx, Ui ui, SharedPreferences prefs, Async async, Overlays ovs, Host host) {
        this.ctx = ctx; this.ui = ui; this.prefs = prefs; this.async = async; this.ovs = ovs; this.host = host;
        if (prefs.getInt("nx_fmt", 1) < MovieRules.CACHE_FMT) {   // ancien cache sans id TMDB : on repart de zero
            SharedPreferences.Editor e = prefs.edit();
            ConfigReceiver.clearMovies(e);
            e.putInt("nx_fmt", MovieRules.CACHE_FMT).apply();
        }
    }

    void buildSpot(ViewGroup parent) {
        // Mise en avant du film : meme emplacement que la meteo, fondu croise (spot*)
        spBox = new LinearLayout(ctx);
        spBox.setOrientation(LinearLayout.VERTICAL);
        spBox.setAlpha(0f);
        spTitle = ui.text("", 28, ui.fMed, 0xFFFFFFFF);
        spTitle.setSingleLine(true); spTitle.setEllipsize(TextUtils.TruncateAt.END); spTitle.setMaxWidth(ui.dp(620));
        spMeta = ui.text("", 13, ui.fReg, 0xE6FFFFFF);
        spMeta.setPadding(0, ui.dp(2), 0, 0);
        spSum = ui.text("", 12, ui.fReg, 0xCCFFFFFF);
        spSum.setMaxLines(3); spSum.setEllipsize(TextUtils.TruncateAt.END); spSum.setMaxWidth(ui.dp(620));
        spSum.setPadding(0, ui.dp(4), 0, 0);
        spBox.addView(spTitle); spBox.addView(spMeta); spBox.addView(spSum);
        FrameLayout.LayoutParams lpSp = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.START);
        lpSp.setMargins(ui.dp(56), ui.dp(76), 0, 0);
        parent.addView(spBox, lpSp);
    }

    void buildRail(ViewGroup parent) {
        // « Nouveau sur Netflix » (au-dessus du dock) ; positions : Home.placeRails()
        nxBox = new LinearLayout(ctx);
        nxBox.setOrientation(LinearLayout.VERTICAL);
        nxBox.setVisibility(View.INVISIBLE);   // cachee au repos : Levels la montre
        nxTab = MovieRules.tab(prefs.getInt("nx_tab", 0));
        nxTabsRow = new LinearLayout(ctx);
        nxTabsRow.setOrientation(LinearLayout.HORIZONTAL);
        nxTabsRow.setPadding(ui.dp(4), 0, 0, ui.dp(2));
        String[] tabNames = {"Netflix", ui.t("Au cinéma", "In theaters"), ui.t("Bientôt", "Coming soon"), ui.t("Ma liste", "My list")};
        for (int i = 0; i < MovieRules.TABS; i++) {
            final int ti = i;
            TextView tb = ui.text(tabNames[i], 12, ui.fMed, 0x99FFFFFF);
            tb.setPadding(ui.dp(12), ui.dp(4), ui.dp(12), ui.dp(4));
            GradientDrawable tg = new GradientDrawable(); tg.setCornerRadius(ui.dp(14)); tg.setColor(0x00FFFFFF);
            tb.setBackground(tg);
            tb.setFocusable(true); tb.setClickable(true);
            // le focus sur une pastille (◀ ▶) change d'onglet tout de suite
            tb.setOnFocusChangeListener(new View.OnFocusChangeListener() { public void onFocusChange(View v, boolean has) {
                if (has && tabNav) selectTab(ti); else styleTabs(); scheduleSpot(); } });   // focus programmatique : pas de changement d'onglet
            tb.setOnClickListener(new View.OnClickListener() { public void onClick(View v) { selectTab(ti); } });
            LinearLayout.LayoutParams tl = new LinearLayout.LayoutParams(-2, -2); tl.rightMargin = ui.dp(6);
            nxTabsRow.addView(tb, tl);
            nxTabs[i] = tb;
        }
        styleTabs();
        nxBox.addView(nxTabsRow);
        nxRow = new LinearLayout(ctx);
        nxRow.setOrientation(LinearLayout.HORIZONTAL);
        nxRow.setClipChildren(false); nxRow.setClipToPadding(false);
        nxRow.setPadding(ui.dp(8), ui.dp(16), ui.dp(8), ui.dp(8));
        nxBox.addView(ui.rail(nxRow), new LinearLayout.LayoutParams(-1, ui.dp(162)));
        FrameLayout.LayoutParams lpN = new FrameLayout.LayoutParams(-1, -2, Gravity.BOTTOM);
        lpN.setMargins(ui.dp(48), 0, ui.dp(48), 0);
        parent.addView(nxBox, lpN);
    }

    String token() { return prefs.getString("tmdb_token", ""); }

    /** Positionne la rangee au-dessus du dock (voir Home.placeRails) ; sa visibilite est geree par Levels. */
    void place(int layout) {
        FrameLayout.LayoutParams b = (FrameLayout.LayoutParams) nxBox.getLayoutParams();
        b.bottomMargin = ui.dp(DockMetrics.nxBase(layout)); nxBox.setLayoutParams(b);
    }

    /** Relance la requete de l'onglet courant si le cache est trop vieux (appele par dataTick, au premier plan). */
    void refresh(long now) {
        String tk = token();
        if (nxTab >= 3) return;   // « Ma liste » est locale : jamais de reseau
        if (tk.length() > 0 && !nxBusy && Fmt.stale(prefs.getLong(MovieRules.tsKey(nxTab), 0), now, NX_TTL)) {
            String key = tk.hashCode() + "/" + nxTab;
            if (!key.equals(nxTryKey) || now - nxTry > Async.RETRY_MS) { nxTryKey = key; nxTry = now; fetch(tk, nxTab); }
        }
    }

    static String str(JSONObject o, String k) { return o.isNull(k) ? "" : o.optString(k, ""); }

    /**
     * Reponse TMDB -> tableau JSON reduit (12 films avec affiche max), pour le cache.
     * Le JSON est lu ici, les regles (filtre, tri, plafond) sont dans MovieRules.select.
     */
    static String slimMovies(String api, boolean upcoming, String today) throws org.json.JSONException {
        JSONArray res = new JSONObject(api).getJSONArray("results"), out = new JSONArray();
        List<Movie> in = new ArrayList<>();
        for (int i = 0; i < res.length(); i++) {
            JSONObject o = res.getJSONObject(i);
            Movie m = new Movie();
            m.title = str(o, "title"); m.overview = str(o, "overview");
            m.poster = str(o, "poster_path"); m.backdrop = str(o, "backdrop_path");
            m.date = str(o, "release_date"); m.vote = o.optDouble("vote_average", 0); m.id = o.optInt("id", 0);
            in.add(m);
        }
        for (Movie m : MovieRules.select(in, upcoming, today)) out.put(movieJson(m));
        return out.toString();
    }

    /** Format du cache reduit (aussi celui de « Ma liste »). */
    static JSONObject movieJson(Movie m) throws org.json.JSONException {
        JSONObject s = new JSONObject();
        s.put("title", m.title); s.put("overview", m.overview);
        s.put("poster_path", m.poster); s.put("backdrop_path", m.backdrop);
        s.put("release_date", m.date); s.put("vote_average", m.vote); s.put("id", m.id);
        return s;
    }

    // ---------- Ma liste (prefs « my_list », plafond 30, survit a clear_tmdb)
    List<Movie> myList() { return parseMovies(prefs.getString(MovieRules.jsonKey(3), null)); }

    boolean inMyList(Movie m) { return MovieRules.listIndex(myList(), m) >= 0; }

    /** Ajoute / retire m de « Ma liste » (message a l'ecran) ; l'onglet est redessine s'il est affiche. */
    void toggleMyList(Movie m) {
        List<Movie> l = myList();
        boolean had = MovieRules.listIndex(l, m) >= 0;
        List<Movie> n = had ? MovieRules.listRemove(l, m) : MovieRules.listAdd(l, m, MovieRules.MY_MAX);
        if (n == null) { ui.toast(ui.t("Ma liste est pleine (30 films)", "My list is full (30 movies)")); return; }
        try {
            JSONArray a = new JSONArray();
            for (Movie x : n) a.put(movieJson(x));
            prefs.edit().putString(MovieRules.jsonKey(3), a.toString()).apply();
        } catch (Throwable e) { return; }
        ui.toast(had ? ui.t("Retiré de ma liste", "Removed from my list") : ui.t("Ajouté à ma liste", "Added to my list"));
        if (nxTab == 3) render();
    }

    static List<Movie> parseMovies(String js) {
        List<Movie> l = new ArrayList<>();
        if (js == null) return l;
        try {
            JSONArray a = new JSONArray(js);
            for (int i = 0; i < a.length() && l.size() < MovieRules.MY_MAX; i++) {
                JSONObject o = a.optJSONObject(i);
                if (o == null) continue;   // entree corrompue : on garde le reste de la liste
                Movie m = new Movie();
                m.title = str(o, "title"); m.overview = str(o, "overview");
                m.poster = MovieRules.imgPath(str(o, "poster_path")); m.backdrop = MovieRules.imgPath(str(o, "backdrop_path"));
                m.date = str(o, "release_date"); m.vote = o.optDouble("vote_average", 0); m.id = o.optInt("id", 0);
                if (!m.poster.isEmpty()) l.add(m);
            }
        } catch (Throwable ignored) { }
        return l;
    }

    // ---------- Mise en avant du film focalise (debounce 120 ms, donnees deja en cache, pas de reseau)
    final Runnable spotRun = new Runnable() { public void run() {
        View f = host.currentFocus();
        int idx = -1;
        if (f != null && ovs.isEmpty()) {
            if (f.getParent() == nxRow) idx = nxRow.indexOfChild(f);
            else if (f.getParent() == nxTabsRow) idx = 0;
        }
        if (idx >= 0 && idx < movies.size()) showSpot(movies.get(idx)); else hideSpot();
    } };

    void scheduleSpot() {
        async.h.removeCallbacks(spotRun); async.h.postDelayed(spotRun, 120);
        async.h.removeCallbacks(bdRun); async.h.postDelayed(bdRun, BD_DEBOUNCE_MS);
    }

    // ---------- Fond = backdrop du film focalise (anti-rebond 400 ms, un seul en attente, cache disque <= 6 fichiers)
    static final long BD_DEBOUNCE_MS = 400;
    static final int BD_CACHE_MAX = 6;
    String bdShown = "";   // chemin du backdrop affiche ou en cours de chargement
    int bdGen = 0;         // chaque nouvelle demande annule la precedente

    final Runnable bdRun = new Runnable() { public void run() {
        if (!ovs.isEmpty() || async.destroyed) return;   // detail ouvert : on garde le fond
        View f = host.currentFocus();
        if (f != null && f.getParent() == nxTabsRow) return;   // onglets : on garde le fond de la rangee
        int idx = f != null && f.getParent() == nxRow ? nxRow.indexOfChild(f) : -1;
        if (idx >= 0 && idx < movies.size() && !movies.get(idx).backdrop.isEmpty()) loadBackdrop(movies.get(idx).backdrop);
        else clearBackdrop();
    } };

    void clearBackdrop() {
        bdGen++;
        if (bdShown.isEmpty()) return;
        bdShown = "";
        host.backdropClear();
    }

    /** onStop : le fond d'ecran est libere, on oublie le backdrop. */
    void dropBackdrop() { bdGen++; bdShown = ""; async.h.removeCallbacks(bdRun); }

    void loadBackdrop(final String path) {
        if (path.equals(bdShown) || !async.fg) return;
        bdShown = path;
        final int gen = ++bdGen;
        host.backdropPrepare();
        final File dir = new File(ctx.getCacheDir(), "backdrops");
        async.bg(new Runnable() { public void run() {
            Bitmap b = null;
            try {
                if (gen == bdGen && async.fg) {
                    File f = new File(dir, MovieRules.posterName(path));
                    if (f.exists()) f.setLastModified(System.currentTimeMillis());
                    else {
                        byte[] d = Net.image(MovieRules.backdropFullUrl(path));
                        dir.mkdirs();
                        File tmp = new File(dir, MovieRules.posterName(path) + ".tmp");
                        FileOutputStream os = new FileOutputStream(tmp);
                        try { os.write(d); } finally { os.close(); }
                        if (!tmp.renameTo(f)) tmp.delete();
                        pruneBackdrops(dir);
                    }
                    if (gen == bdGen) {
                        BitmapFactory.Options o = new BitmapFactory.Options();
                        o.inJustDecodeBounds = true;
                        BitmapFactory.decodeFile(f.getPath(), o);
                        if (MovieRules.tooBig(o.outWidth, o.outHeight)) { f.delete(); throw new java.io.IOException("image trop grande"); }
                        o.inSampleSize = MovieRules.sample(o.outWidth, 1280);
                        o.inJustDecodeBounds = false;
                        o.inPreferredConfig = Bitmap.Config.RGB_565;
                        b = BitmapFactory.decodeFile(f.getPath(), o);
                        if (b == null) f.delete();
                    }
                }
            } catch (Throwable ignored) { }
            final Bitmap fb = b;
            async.ui.post(new Runnable() { public void run() {
                if (fb == null) { if (gen == bdGen) bdShown = ""; return; }
                if (async.destroyed || gen != bdGen) { fb.recycle(); return; }
                host.backdropReady(fb);
            } });
        } });
    }

    /** Garde les BD_CACHE_MAX backdrops les plus recents. */
    static void pruneBackdrops(File dir) {
        File[] fs = dir.listFiles();
        if (fs == null || fs.length <= BD_CACHE_MAX) return;
        java.util.Arrays.sort(fs, new java.util.Comparator<File>() {
            public int compare(File a, File b) { return Long.compare(b.lastModified(), a.lastModified()); } });
        for (int i = BD_CACHE_MAX; i < fs.length; i++) fs[i].delete();
    }

    void showSpot(Movie m) {
        if (m != spMovie) {
            spMovie = m;
            String meta = MovieRules.spotMeta(m, nxTab, ui.fr);
            spTitle.setText(m.title); spMeta.setText(meta); spSum.setText(m.overview);
        }
        if (!spOn) {
            spOn = true;
            spBox.animate().alpha(1f).setDuration(200).start();
            host.onSpot(true);
        }
    }

    void hideSpot() {
        if (!spOn) return;
        spOn = false; spMovie = null;
        spBox.animate().alpha(0f).setDuration(200).start();
        host.onSpot(false);
    }

    /** Pastilles : fond sombre translucide ; active : verre clair ; avec le focus : couleur d'accent. */
    void styleTabs() {
        for (int i = 0; i < MovieRules.TABS; i++) {
            TextView v = nxTabs[i];
            boolean foc = v.hasFocus(), act = i == nxTab;
            ((GradientDrawable) v.getBackground()).setColor(foc ? Ui.ACCENT : (act ? 0xD9FFFFFF : ui.theme.p.pillFill));
            v.setTextColor(foc || act ? 0xFF111111 : 0xFFFFFFFF);
        }
    }

    void selectTab(int i) {
        if (i != nxTab) {
            nxTab = i;
            prefs.edit().putInt("nx_tab", i).apply();
            render();    // cache de l'onglet d'abord
            host.dataTick();        // puis reseau si le cache a plus de 6 h
        }
        styleTabs();
    }

    /** Affiche la rangee depuis le cache des prefs ; sans jeton ou sans donnees la section est masquee. */
    void render() {
        String js = token().length() > 0 ? prefs.getString(MovieRules.jsonKey(nxTab), null) : null;
        String sig = js == null ? "" : nxTab + js;
        if (sig.equals(nxShown)) { host.placeRails(); return; }
        nxShown = sig;
        movies = parseMovies(js);
        releasePosters();
        View before = host.currentFocus();
        int bi = before != null && before.getParent() == nxRow ? nxRow.indexOfChild(before) : -1;
        nxRow.removeAllViews();
        posterIv.clear();
        for (int i = 0; i < movies.size(); i++) nxRow.addView(posterTile(movies.get(i)));
        host.fixRowFocus(before, bi, nxRow);
        scheduleSpot();
        host.placeRails();
        loadPosters();
    }

    View posterTile(final Movie m) {
        FrameLayout f = ui.roundedFrame(8, 0xFF1E1E1E);
        ImageView iv = new ImageView(ctx);
        iv.setScaleType(ImageView.ScaleType.CENTER_CROP);
        f.addView(iv, new FrameLayout.LayoutParams(-1, -1));
        posterIv.add(iv);
        int badge = nxTab >= 2 ? MovieRules.releaseBadge(m.date, new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date())) : 0;
        if (badge > 0) {   // « Bientot » et « Ma liste » : sortie aujourd'hui / deja sortie
            TextView bd = ui.text(badge == 1 ? ui.t("Sortie aujourd'hui", "Out today") : ui.t("Sorti", "Out now"), 7, ui.fMed, 0xFF111111);
            bd.setShadowLayer(0, 0, 0, 0); bd.setMaxWidth(ui.dp(80));
            bd.setPadding(ui.dp(5), ui.dp(2), ui.dp(5), ui.dp(2));
            GradientDrawable bg = new GradientDrawable(); bg.setCornerRadius(ui.dp(6)); bg.setColor(badge == 1 ? Ui.ACCENT : 0xE6FFFFFF);
            bd.setBackground(bg);
            FrameLayout.LayoutParams lb = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.START);
            lb.setMargins(ui.dp(4), 0, 0, ui.dp(4));
            f.addView(bd, lb);
        }
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ui.dp(92), ui.dp(138)); lp.setMargins(ui.dp(5), 0, ui.dp(5), 0);
        f.setLayoutParams(lp);
        f.setTag(8);
        f.setContentDescription(m.title.isEmpty() ? ui.t("Film", "Movie") : m.title);
        ui.wire(f, new Runnable() { public void run() { host.onMovieClick(m); } }, 1.1f);
        return f;
    }

    void fetch(final String tk, final int tab) {
        nxBusy = true;
        final String today = new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date());
        final String url = MovieRules.listUrl(tab, today);
        final File dir = new File(ctx.getCacheDir(), "posters");
        async.bg(new Runnable() { public void run() {
            try {
                if (async.fg) {
                    String slim = slimMovies(Net.json(url, tk), tab == 2, today);
                    List<Movie> l = parseMovies(slim);
                    if (!tk.equals(token())) return;   // jeton change / efface pendant la requete
                    if (!l.isEmpty()) {
                        prefs.edit().putString(MovieRules.jsonKey(tab), slim).putLong(MovieRules.tsKey(tab), System.currentTimeMillis()).apply();
                        HashMap<String, Boolean> keep = new HashMap<>();   // purge : on garde les affiches des 4 onglets (Ma liste comprise)
                        for (int k = 0; k < MovieRules.TABS; k++)
                            for (Movie m : parseMovies(prefs.getString(MovieRules.jsonKey(k), null))) keep.put(MovieRules.posterName(m.poster), true);
                        File[] fs = dir.listFiles();
                        if (fs != null) for (File f : fs) if (!keep.containsKey(f.getName())) f.delete();
                    }
                }
            } catch (Throwable ignored) { }
            finally { async.ui.post(new Runnable() { public void run() { nxBusy = false; if (!async.destroyed) { render(); loadPosters(); host.dataTick(); } } }); }
        } });
    }

    /** Charge (disque, sinon reseau) les affiches qui n'ont pas encore d'image ; RGB_565. */
    void loadPosters() {
        if (posterBusy || async.destroyed || movies.isEmpty()) return;
        final List<String> paths = new ArrayList<>();
        boolean any = false;
        for (int i = 0; i < movies.size(); i++) {
            boolean need = i < posterIv.size() && posterIv.get(i).getDrawable() == null;
            paths.add(need ? movies.get(i).poster : null);
            any |= need;
        }
        if (!any) return;
        final int gen = posterGen;
        final File dir = new File(ctx.getCacheDir(), "posters");
        posterBusy = true;
        async.bg(new Runnable() { public void run() {
            try {
                dir.mkdirs();
                for (int i = 0; i < paths.size() && !async.destroyed; i++) {
                    String p = paths.get(i);
                    if (p == null) continue;
                    File f = new File(dir, MovieRules.posterName(p));
                    if (!f.exists()) {
                        if (!async.fg) continue;
                        try {
                            byte[] d = Net.image(MovieRules.posterUrl(p));
                            File tmp = new File(dir, MovieRules.posterName(p) + ".tmp");
                            FileOutputStream os = new FileOutputStream(tmp);
                            try { os.write(d); } finally { os.close(); }
                            if (!tmp.renameTo(f)) tmp.delete();
                        } catch (Throwable e) { continue; }
                    }
                    BitmapFactory.Options o = new BitmapFactory.Options();
                    o.inJustDecodeBounds = true;
                    BitmapFactory.decodeFile(f.getPath(), o);
                    if (MovieRules.tooBig(o.outWidth, o.outHeight)) { f.delete(); continue; }
                    o.inSampleSize = MovieRules.sample(o.outWidth, 185);
                    o.inJustDecodeBounds = false;
                    o.inPreferredConfig = Bitmap.Config.RGB_565;
                    final Bitmap b = BitmapFactory.decodeFile(f.getPath(), o);
                    if (b == null) { f.delete(); continue; }
                    final int idx = i;
                    async.ui.post(new Runnable() { public void run() { setPoster(idx, b, gen); } });
                }
            } catch (Throwable ignored) { }
            finally { async.ui.post(new Runnable() { public void run() { posterBusy = false; } }); }
        } });
    }

    void setPoster(int i, Bitmap b, int gen) {
        if (async.destroyed || gen != posterGen || i >= posterIv.size() || posterIv.get(i).getDrawable() != null) { b.recycle(); return; }
        posterIv.get(i).setImageBitmap(b);
    }

    /** Libere les bitmaps d'affiches (onStop, ou liste remplacee) ; les resultats en vol sont ignores. */
    void releasePosters() {
        posterGen++;
        for (ImageView v : posterIv) {
            Drawable d = v.getDrawable(); v.setImageDrawable(null);
            if (d instanceof BitmapDrawable) { Bitmap b = ((BitmapDrawable) d).getBitmap(); if (b != null) b.recycle(); }
        }
    }
}
