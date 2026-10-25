package tv.zenhome;

import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.List;
import java.util.Locale;

/** Regles pures des films TMDB : chemins d'images, filtre/tri « Bientot », caches, affiches, libelles. */
final class MovieRules {
    private MovieRules() { }

    static String imgPath(String p) { return p.matches("/[A-Za-z0-9_-]+\\.(jpg|png)") ? p : ""; }

    /**
     * Films bruts de la reponse TMDB -> films retenus pour le cache (12 avec affiche max, chemins d'images assainis).
     * upcoming : on ne garde que release_date >= today (TMDB renvoie parfois d'anciennes ressorties), tri croissant stable.
     */
    static List<Movie> select(List<Movie> in, boolean upcoming, String today) {
        List<Movie> l = new ArrayList<>();
        for (Movie m : in) {
            if (imgPath(m.poster).isEmpty()) continue;
            if (upcoming && m.date.compareTo(today) < 0) continue;   // vide ou passee
            l.add(m);
        }
        if (upcoming) Collections.sort(l, new Comparator<Movie>() {
            public int compare(Movie a, Movie b) { return a.date.compareTo(b.date); } });
        List<Movie> out = new ArrayList<>();
        for (int i = 0; i < l.size() && out.size() < 12; i++) {
            Movie m = l.get(i), s = new Movie();
            s.title = cut(m.title, TITLE_MAX); s.overview = cut(m.overview, OVERVIEW_MAX);
            s.poster = imgPath(m.poster); s.backdrop = imgPath(m.backdrop);
            s.date = m.date; s.vote = m.vote; s.id = m.id;
            out.add(s);
        }
        return out;
    }

    /** Format du cache reduit des films (2 : ajout de l'id TMDB). Un cache plus ancien est efface au demarrage. */
    static final int CACHE_FMT = 2;

    static final int TABS = 4;      // 0 Netflix, 1 Au cinema, 2 Bientot, 3 Ma liste (locale, sans reseau)
    static final int MY_MAX = 30;   // plafond de « Ma liste »

    /** Onglet memorise (nx_tab) ramene dans 0..3. */
    static int tab(int v) { return Math.max(0, Math.min(TABS - 1, v)); }

    /** Cles des caches. L'onglet 3 (« Ma liste ») est a part : clear_tmdb ne l'efface pas (ConfigReceiver ne boucle que sur 0..2). */
    static final int TITLE_MAX = 200, OVERVIEW_MAX = 1000, PIXELS_MAX = 4_000_000;

    /** Texte distant plafonne (caches et « Ma liste »). */
    static String cut(String s, int max) { return s == null ? "" : (s.length() > max ? s.substring(0, max) : s); }

    /** Image trop grande pour etre decodee (largeur x hauteur lues avant le decodage). */
    static boolean tooBig(int w, int h) { return w <= 0 || h <= 0 || (long) w * h > PIXELS_MAX; }

    /** Onglet voisin (dir -1 / +1) borne a 0..TABS-1 ; from hors bornes : inchange. */
    static int tabStep(int from, int dir) { return from < 0 || from >= TABS ? from : Math.max(0, Math.min(TABS - 1, from + dir)); }

    static String jsonKey(int tab) { return tab == 0 ? "nx_json" : (tab == 1 ? "nc_json" : (tab == 2 ? "nu_json" : "my_list")); }
    static String tsKey(int tab) { return tab == 0 ? "nx_ts" : (tab == 1 ? "nc_ts" : (tab == 2 ? "nu_ts" : "ml_ts")); }

    /** inSampleSize (puissance de 2) tel que l'image decodee reste >= target de large. */
    static int sample(int w, int target) {
        int s = 1;
        while (w / (s * 2) >= target) s *= 2;
        return s;
    }

    static String posterName(String path) { return Integer.toHexString(path.hashCode()) + ".img"; }

    static String posterUrl(String path) { return "https://image.tmdb.org/t/p/w185" + path; }

    static String backdropUrl(String path) { return "https://image.tmdb.org/t/p/w780" + path; }

    /** Backdrop pour le fond d'ecran (1280 de large). */
    static String backdropFullUrl(String path) { return "https://image.tmdb.org/t/p/w1280" + path; }

    /** URL TMDB de l'onglet (0 Netflix, 1 Au cinema, 2 Bientot). */
    static String listUrl(int tab, String today) {
        return tab == 0
                ? "https://api.themoviedb.org/3/discover/movie?watch_region=FR&with_watch_providers=8"
                + "&with_watch_monetization_types=flatrate&language=fr-FR&sort_by=primary_release_date.desc"
                + "&primary_release_date.lte=" + today + "&vote_count.gte=20&page=1"
                : "https://api.themoviedb.org/3/movie/" + (tab == 1 ? "now_playing" : "upcoming") + "?language=fr-FR&region=FR&page=1";
    }

    static String fmtDate(String ymd, String pattern, Locale loc) {
        try { return new SimpleDateFormat(pattern, loc).format(new SimpleDateFormat("yyyy-MM-dd", Locale.US).parse(ymd)); }
        catch (Exception e) { return ""; }
    }

    private static String withVote(Movie m, String meta, boolean fr) {
        if (m.vote > 0) meta += (meta.isEmpty() ? "" : "  ·  ") + "★ " + String.format(fr ? Locale.FRANCE : Locale.US, "%.1f", m.vote) + " / 10";
        return meta;
    }

    /** Ligne meta de la mise en avant (onglet 2 : « Sortie le d MMMM »). */
    static String spotMeta(Movie m, int tab, boolean fr) {
        String meta;
        if (tab == 2) {
            meta = fmtDate(m.date, "d MMMM", fr ? Locale.FRANCE : Locale.US);
            if (!meta.isEmpty()) meta = (fr ? "Sortie le " : "Releases ") + meta;
        } else meta = m.date.length() >= 4 ? m.date.substring(0, 4) : "";
        return withVote(m, meta, fr);
    }

    /** Ligne meta du detail (onglet 2 : date complete, sinon l'annee). */
    static String detailMeta(Movie m, int tab, boolean fr) {
        String meta = m.date.length() >= 4 ? m.date.substring(0, 4) : "";
        if (tab == 2) {   // Bientot : date de sortie complete
            try {
                Date rd = new SimpleDateFormat("yyyy-MM-dd", Locale.US).parse(m.date);
                meta = (fr ? "Sortie le " : "Releases ") + new SimpleDateFormat("d MMMM yyyy", fr ? Locale.FRANCE : Locale.US).format(rd);
            } catch (Exception ignored) { }
        }
        return withVote(m, meta, fr);
    }

    // ---------- Bande-annonce

    /** Une video de la reponse TMDB /movie/{id}/videos. */
    static final class Video {
        final String key, type, site;
        final boolean official;
        Video(String key, String type, String site, boolean official) { this.key = key; this.type = type; this.site = site; this.official = official; }
    }

    /** Cle YouTube valide (11 caracteres [A-Za-z0-9_-]), sinon "". Seule donnee distante qui finit dans un intent. */
    static String youtubeKey(String k) { return k != null && k.matches("[A-Za-z0-9_-]{11}") ? k : ""; }

    /** Bande-annonce a lancer : type Trailer, site YouTube, cle valide ; officielle en priorite, sinon la premiere. "" si aucune. */
    static String pickTrailer(List<Video> vs) {
        String first = "";
        for (Video v : vs) {
            if (!"Trailer".equals(v.type) || !"YouTube".equals(v.site) || youtubeKey(v.key).isEmpty()) continue;
            if (v.official) return v.key;
            if (first.isEmpty()) first = v.key;
        }
        return first;
    }

    /** URL TMDB des videos d'un film (lang : fr-FR puis en-US). */
    static String videosUrl(int id, String lang) { return "https://api.themoviedb.org/3/movie/" + id + "/videos?language=" + lang; }

    static String watchUrl(String key) { return "https://www.youtube.com/watch?v=" + key; }

    // ---------- Ma liste

    /** Rang de m dans la liste (meme id TMDB, ou meme titre + affiche si l'id est inconnu), ou -1. */
    static int listIndex(List<Movie> l, Movie m) {
        for (int i = 0; i < l.size(); i++) {
            Movie x = l.get(i);
            if (m.id > 0 ? x.id == m.id : (x.id == 0 && x.title.equals(m.title) && x.poster.equals(m.poster))) return i;
        }
        return -1;
    }

    /** Ajoute m en tete (sans doublon) ; renvoie null si la liste est pleine. */
    static List<Movie> listAdd(List<Movie> l, Movie m, int max) {
        if (listIndex(l, m) >= 0) return l;
        if (l.size() >= max) return null;
        List<Movie> out = new ArrayList<>(l);
        out.add(0, m);
        return out;
    }

    static List<Movie> listRemove(List<Movie> l, Movie m) {
        List<Movie> out = new ArrayList<>(l);
        int i = listIndex(out, m);
        if (i >= 0) out.remove(i);
        return out;
    }

    /** Badge d'une affiche : 1 « Sortie aujourd'hui », 2 « Sorti » (date passee), 0 sinon (date vide ou a venir). today : yyyy-MM-dd. */
    static int releaseBadge(String date, String today) {
        if (date.length() != 10) return 0;
        int c = date.compareTo(today);
        return c == 0 ? 1 : (c < 0 ? 2 : 0);
    }

    // ---------- Bandeau « sort aujourd'hui » (Ma liste)

    /** Titres des films de la liste dont la date de sortie est today (yyyy-MM-dd), dans l'ordre de la liste. */
    static List<String> releasingToday(List<Movie> l, String today) {
        List<String> out = new ArrayList<>();
        for (Movie m : l) if (m.date.length() == 10 && m.date.equals(today) && !m.title.isEmpty()) out.add(m.title);
        return out;
    }

    /** Le bandeau doit-il s'afficher ? Au moins une sortie aujourd'hui et pas deja montre ce jour-la. */
    static boolean bannerDue(String shownDay, String today, int count) { return count > 0 && !today.equals(shownDay); }

    /** « Godzilla Minus Zero sort aujourd'hui » / « 2 films de ta liste sortent aujourd'hui » ; vide s'il n'y en a pas. */
    static String bannerText(List<String> titles, boolean fr) {
        if (titles.isEmpty()) return "";
        if (titles.size() == 1) return titles.get(0) + (fr ? " sort aujourd'hui" : " is out today");
        return fr ? titles.size() + " films de ta liste sortent aujourd'hui" : titles.size() + " movies on your list are out today";
    }
}
