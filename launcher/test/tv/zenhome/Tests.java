package tv.zenhome;

import java.net.URL;
import java.util.ArrayList;
import java.util.Calendar;
import java.util.List;

/** Lanceur de tests maison (sans JUnit). Sortie 1 si une assertion echoue. Valeurs figees = non-regression. */
public class Tests {
    static int count = 0, failed = 0;

    static void ok(String name, boolean cond) {
        count++;
        if (!cond) { failed++; System.out.println("ECHEC : " + name); }
    }

    static void eq(String name, Object expected, Object actual) {
        count++;
        if (expected == null ? actual != null : !expected.equals(actual)) {
            failed++;
            System.out.println("ECHEC : " + name + "\n   attendu : <" + expected + ">\n   obtenu  : <" + actual + ">");
        }
    }

    public static void main(String[] args) {
        net(); fmt(); wmo(); greeting(); dock(); movieRules(); configRules(); nav(); trailer(); myList(); hardening(); inputs(); sleep(); weatherRules(); keyRules(); settingsKey(); gauge(); standby(); iconRules(); themeRules(); menuRules(); recent(); nightLabels(); attribution(); banner();
        System.out.println(count + " assertions, " + failed + " echec(s)");
        if (failed > 0) System.exit(1);
    }

    // ---------- Net
    static boolean allowed(String u) {
        try { return Net.allowed(new URL(u)); } catch (Exception e) { return false; }
    }

    static void net() {
        ok("net: open-meteo", allowed("https://api.open-meteo.com/v1/forecast"));
        ok("net: tmdb api", allowed("https://api.themoviedb.org/3/movie/upcoming"));
        ok("net: tmdb image", allowed("https://image.tmdb.org/t/p/w185/a.jpg"));
        ok("net: port 443 explicite", allowed("https://api.open-meteo.com:443/x"));
        ok("net: http refuse", !allowed("http://api.open-meteo.com/x"));
        ok("net: port 8443 refuse", !allowed("https://api.open-meteo.com:8443/x"));
        ok("net: userinfo refuse", !allowed("https://user@api.open-meteo.com/x"));
        ok("net: hote suffixe refuse", !allowed("https://api.open-meteo.com.evil.com/x"));
        ok("net: hote prefixe refuse", !allowed("https://evil-api.open-meteo.com/x"));
        ok("net: majuscules refusees", !allowed("https://API.OPEN-METEO.COM/x"));
    }

    // ---------- Fmt
    static void fmt() {
        eq("kb: ligne normale", 3921624L, Fmt.kb("MemTotal:        3921624 kB"));
        eq("kb: sans valeur", -1L, Fmt.kb("MemTotal:"));
        eq("kb: non numerique", -1L, Fmt.kb("MemTotal: abc kB"));
        eq("kb: vide", -1L, Fmt.kb(""));
        eq("deg arrondi", "22°", Fmt.deg(21.5));
        eq("deg negatif", "-1°", Fmt.deg(-1.4));
        eq("deg zero negatif", "0°", Fmt.deg(-0.4));
        ok("stale: ts 0", Fmt.stale(0, 1000, 500));
        ok("stale: futur", Fmt.stale(2000, 1000, 500));
        ok("stale: trop vieux", Fmt.stale(100, 1000, 500));
        ok("stale: exactement ttl", !Fmt.stale(500, 1000, 500));
        ok("stale: frais", !Fmt.stale(900, 1000, 500));
    }

    // ---------- Wmo
    static void w(int code, String icon, String fr, String en) {
        eq("wmo " + code + " icone", icon, Wmo.of(code, true)[0]);
        eq("wmo " + code + " fr", fr, Wmo.of(code, true)[1]);
        eq("wmo " + code + " en", en, Wmo.of(code, false)[1]);
        eq("wmo " + code + " icone en", icon, Wmo.of(code, false)[0]);
    }

    static void wmo() {
        w(0, "☀", "Ensoleillé", "Clear");
        w(1, "⛅", "Peu nuageux", "Partly cloudy");
        w(2, "⛅", "Peu nuageux", "Partly cloudy");
        w(3, "☁", "Couvert", "Overcast");
        w(4, "☁", "Variable", "Variable");
        w(44, "☁", "Variable", "Variable");
        w(45, "≋", "Brouillard", "Fog");
        w(48, "≋", "Brouillard", "Fog");
        w(50, "☁", "Variable", "Variable");
        w(51, "☂", "Bruine", "Drizzle");
        w(57, "☂", "Bruine", "Drizzle");
        w(58, "☁", "Variable", "Variable");
        w(61, "☂", "Pluie", "Rain");
        w(67, "☂", "Pluie", "Rain");
        w(68, "☁", "Variable", "Variable");
        w(71, "❄", "Neige", "Snow");
        w(77, "❄", "Neige", "Snow");
        w(78, "☁", "Variable", "Variable");
        w(80, "☂", "Averses", "Showers");
        w(82, "☂", "Averses", "Showers");
        w(83, "☁", "Variable", "Variable");
        w(85, "❄", "Averses de neige", "Snow showers");
        w(86, "❄", "Averses de neige", "Snow showers");
        w(87, "☁", "Variable", "Variable");
        w(94, "☁", "Variable", "Variable");
        w(95, "⚡", "Orage", "Thunderstorm");
        w(99, "⚡", "Orage", "Thunderstorm");
        w(100, "⚡", "Orage", "Thunderstorm");
        eq("phrase 4 (defaut pluie)", "Il pleut : parfait pour un film", Wmo.phrase(4, 0, "", true));
        eq("phrase 100 (orage)", "Orage dehors, reste au chaud", Wmo.phrase(100, 0, "", true));
        eq("phrase 61 en", "Rainy out: perfect for a movie", Wmo.phrase(61, 0, "", false));
        eq("phrase 1", "Entre nuages et éclaircies", Wmo.phrase(1, 1, "", true));
        eq("phrase 3", "Ciel gris, ambiance série", Wmo.phrase(3, 1, "", true));
        eq("phrase brouillard matin", "Brouillard ce matin, reste au chaud", Wmo.phrase(45, 0, "", true));
        eq("phrase brouillard apres-midi", "Brouillard dehors, reste au chaud", Wmo.phrase(48, 1, "", true));
        eq("phrase neige 73", "Il neige : plaid et film ?", Wmo.phrase(73, 2, "", true));
        eq("phrase neige 85", "Il neige : plaid et film ?", Wmo.phrase(85, 2, "", true));
        eq("phrase neige 86 en", "Snowing: blanket and a movie?", Wmo.phrase(86, 2, "", false));
        eq("phrase soleil matin ville", "Grand soleil sur Paris", Wmo.phrase(0, 0, "Paris", true));
        eq("phrase soleil apres-midi ville en", "Bright sunshine in Paris", Wmo.phrase(0, 1, "Paris", false));
        eq("phrase soleil sans ville", "Grand soleil aujourd'hui", Wmo.phrase(0, 0, "", true));
        eq("phrase soleil ville 23 car. (fr, total 40)", "Grand soleil sur " + rep('v', 23), Wmo.phrase(0, 0, rep('v', 23), true));
        eq("phrase soleil ville 24 car. (fr, total 41)", "Grand soleil aujourd'hui", Wmo.phrase(0, 0, rep('v', 24), true));
        eq("phrase soleil ville 25 car. (en, total 40)", "Bright sunshine in " + rep('v', 21), Wmo.phrase(0, 1, rep('v', 21), false));
        eq("phrase soleil ville 22 car. (en, total 41)", "Bright sunshine today", Wmo.phrase(0, 1, rep('v', 22), false));
        eq("phrase ciel degage soir", "Ciel dégagé ce soir", Wmo.phrase(0, 2, "Paris", true));
        eq("phrase ciel degage nuit", "Ciel dégagé cette nuit", Wmo.phrase(0, 3, "Paris", true));
        eq("phrase ciel degage nuit en", "Clear skies overnight", Wmo.phrase(0, 3, "Paris", false));
    }

    static String rep(char c, int n) {
        StringBuilder b = new StringBuilder();
        for (int i = 0; i < n; i++) b.append(c);
        return b.toString();
    }

    // ---------- Greeting
    static void greeting() {
        ok("nuit 22h", Greeting.night(22)); ok("nuit 23h", Greeting.night(23)); ok("nuit 0h", Greeting.night(0));
        ok("nuit 5h", Greeting.night(5)); ok("jour 6h", !Greeting.night(6)); ok("jour 21h", !Greeting.night(21));
        int[][] slots = {{0, 3}, {4, 3}, {5, 0}, {11, 0}, {12, 1}, {17, 1}, {18, 2}, {21, 2}, {22, 3}, {23, 3}};
        for (int[] s : slots) eq("slotOf(" + s[0] + ")", s[1], Greeting.slotOf(s[0]));
        eq("GREET matin fr", "Bonjour", Greeting.GREET[0][0]);
        eq("GREET nuit en", "Good night", Greeting.GREET[3][1]);
        eq("LINES: 4 tranches", 4, Greeting.LINES.length);

        // sans meteo, jour ordinaire (mercredi) : graine (doy*4 + slot) % taille
        eq("phrase determinisme 1", Greeting.phrase(1, Calendar.WEDNESDAY, 0, -1, "", true), Greeting.phrase(1, Calendar.WEDNESDAY, 0, -1, "", true));
        eq("phrase matin fr", "On commence en douceur ?", Greeting.phrase(1, Calendar.WEDNESDAY, 0, -1, "", true));
        eq("phrase matin en", "Let's start gently?", Greeting.phrase(1, Calendar.WEDNESDAY, 0, -1, "", false));
        eq("phrase apres-midi", "Prends un moment pour toi", Greeting.phrase(1, Calendar.WEDNESDAY, 1, -1, "", true));
        eq("phrase soir", "Une soirée film ?", Greeting.phrase(3, Calendar.WEDNESDAY, 2, -1, "", true));
        eq("phrase nuit", "Pense à dormir un peu", Greeting.phrase(1, Calendar.WEDNESDAY, 3, -1, "", true));
        for (int slot = 0; slot < 4; slot++)
            for (int doy = 1; doy <= 366; doy++) {
                String p = Greeting.phrase(doy, Calendar.WEDNESDAY, slot, -1, "", true);
                if (p == null || p.isEmpty()) { ok("phrase non vide " + doy + "/" + slot, false); }
            }

        // jours speciaux : seulement si (doy + slot) pair
        eq("vendredi soir, jour pair", "Enfin le week-end", Greeting.phrase(2, Calendar.FRIDAY, 2, -1, "", true));
        eq("vendredi soir en", "Weekend at last", Greeting.phrase(2, Calendar.FRIDAY, 2, -1, "", false));
        eq("vendredi soir, jour impair", "Une soirée film ?", Greeting.phrase(3, Calendar.FRIDAY, 2, -1, "", true));
        eq("samedi matin, pair", "Bon samedi", Greeting.phrase(2, Calendar.SATURDAY, 0, -1, "", true));
        eq("samedi matin, impair", "Café et nouveautés ?", Greeting.phrase(3, Calendar.SATURDAY, 0, -1, "", true));
        eq("samedi nuit, pair: pas special", "Pense à dormir un peu", Greeting.phrase(1, Calendar.SATURDAY, 3, -1, "", true));
        eq("dimanche apres-midi, pair", "Bon dimanche", Greeting.phrase(1, Calendar.SUNDAY, 1, -1, "", true));
        eq("dimanche en", "Happy Sunday", Greeting.phrase(1, Calendar.SUNDAY, 1, -1, "", false));
        eq("lundi matin, pair", "Courage pour ce lundi", Greeting.phrase(2, Calendar.MONDAY, 0, -1, "", true));
        eq("lundi matin en", "Hang in there, it's Monday", Greeting.phrase(2, Calendar.MONDAY, 0, -1, "", false));
        eq("lundi apres-midi: pas special", "Prends un moment pour toi", Greeting.phrase(1, Calendar.MONDAY, 1, -1, "", true));

        // meteo : prioritaire sur les lignes, mais pas sur un jour special
        eq("meteo prioritaire", "Il pleut : parfait pour un film", Greeting.phrase(1, Calendar.WEDNESDAY, 0, 61, "", true));
        eq("meteo soleil + ville", "Grand soleil sur Lyon", Greeting.phrase(1, Calendar.WEDNESDAY, 0, 0, "Lyon", true));
        eq("meteo ville longue", "Grand soleil aujourd'hui", Greeting.phrase(1, Calendar.WEDNESDAY, 0, 0, rep('x', 30), true));
        eq("jour special bat la meteo", "Bon samedi", Greeting.phrase(2, Calendar.SATURDAY, 0, 61, "", true));
        eq("meteo si jour special impair", "Il pleut : parfait pour un film", Greeting.phrase(3, Calendar.SATURDAY, 0, 61, "", true));
    }

    // ---------- Nav
    static Nav.State go(Nav.State s, Nav.Key k, boolean films, boolean weather, boolean edit, boolean first) {
        return Nav.next(s, k, new Nav.Flags(films, weather, edit, first));
    }

    static void nav() {
        Nav.State R = Nav.State.REST, A = Nav.State.APPS, F = Nav.State.FILMS, T = Nav.State.TABS, P = Nav.State.TOP, W = Nav.State.WEATHER;
        Nav.Key U = Nav.Key.UP, D = Nav.Key.DOWN, L = Nav.Key.LEFT_EDGE, Rt = Nav.Key.RIGHT_EDGE, B = Nav.Key.BACK;
        // tous drapeaux actifs (films, meteo, hors edition, 1er raccourci)
        eq("REST haut -> films", F, go(R, U, true, true, false, false));
        eq("REST bas -> applis", A, go(R, D, true, true, false, false));
        eq("REST retour", R, go(R, B, true, true, false, false));
        eq("REST bords", R, go(R, L, true, true, false, false));
        eq("REST bord droit", R, go(R, Rt, true, true, false, false));
        eq("APPS haut -> repos", R, go(A, U, true, true, false, false));
        eq("APPS bas bloque", A, go(A, D, true, true, false, false));
        eq("APPS retour", R, go(A, B, true, true, false, false));
        eq("APPS bords", A, go(A, L, true, true, false, false));
        eq("FILMS haut -> onglets", T, go(F, U, true, true, false, false));
        eq("FILMS bas -> repos", R, go(F, D, true, true, false, false));
        eq("FILMS retour", R, go(F, B, true, true, false, false));
        eq("FILMS bords", F, go(F, Rt, true, true, false, false));
        eq("TABS haut -> barre", P, go(T, U, true, true, false, false));
        eq("TABS bas -> films", F, go(T, D, true, true, false, false));
        eq("TABS retour", R, go(T, B, true, true, false, false));
        eq("TABS bords", T, go(T, L, true, true, false, false));
        eq("TOP bas -> onglets", T, go(P, D, true, true, false, false));
        eq("TOP haut bloque", P, go(P, U, true, true, false, false));
        eq("TOP gauche 1er -> meteo", W, go(P, L, true, true, false, true));
        eq("TOP bord droit", P, go(P, Rt, true, true, false, false));
        eq("TOP retour", R, go(P, B, true, true, false, true));
        eq("WEATHER droite -> barre", P, go(W, Rt, true, true, false, false));
        eq("WEATHER bas -> onglets", T, go(W, D, true, true, false, false));
        eq("WEATHER retour", R, go(W, B, true, true, false, false));
        eq("WEATHER gauche bloque", W, go(W, L, true, true, false, false));
        eq("WEATHER haut bloque", W, go(W, U, true, true, false, false));
        // sans films
        eq("sans films: REST haut -> barre", P, go(R, U, false, true, false, false));
        eq("sans films: TOP bas -> repos", R, go(P, D, false, true, false, false));
        eq("sans films: WEATHER bas -> repos", R, go(W, D, false, true, false, false));
        eq("sans films: FILMS -> repos", R, go(F, U, false, true, false, false));
        eq("sans films: TABS -> repos", R, go(T, D, false, true, false, false));
        // sans meteo
        eq("sans meteo: TOP gauche 1er reste", P, go(P, L, true, false, false, true));
        eq("sans meteo: WEATHER -> repos", R, go(W, Rt, true, false, false, false));
        eq("sans meteo: REST haut -> films", F, go(R, U, true, false, false, false));
        // 1er raccourci
        eq("TOP gauche pas 1er reste", P, go(P, L, true, true, false, false));
        eq("hors TOP le drapeau 1er est sans effet", F, go(F, L, true, true, false, true));
        // edition : films desactives
        eq("edition: REST haut -> barre", P, go(R, U, true, true, true, false));
        eq("edition: TOP bas -> repos", R, go(P, D, true, true, true, false));
        eq("edition: REST bas -> applis", A, go(R, D, true, true, true, false));
        eq("edition: FILMS -> repos", R, go(F, D, true, true, true, false));
        eq("edition: meteo reste accessible", W, go(P, L, true, true, true, true));
        // ambiance
        Nav.State Am = Nav.State.AMBIENT;
        for (Nav.State s : Nav.State.values()) eq("idle " + s, s == Am ? Am : Am, go(s, Nav.Key.IDLE, true, true, false, false));
        eq("idle refuse en edition", R, Nav.next(R, Nav.Key.IDLE, new Nav.Flags(true, true, true, false)));
        eq("idle refuse (overlay / hors premier plan)", F, Nav.next(F, Nav.Key.IDLE, new Nav.Flags(true, true, false, false, false)));
        for (Nav.Key k : new Nav.Key[]{U, D, L, Rt, Nav.Key.IDLE}) eq("ambiance consomme " + k, Am, go(Am, k, true, true, false, false));
        eq("ambiance retour", R, go(Am, B, true, true, false, false));
        eq("reveil: meme niveau", F, Nav.wake(F, new Nav.Flags(true, true, false, false)));
        eq("reveil: films disparus", R, Nav.wake(F, new Nav.Flags(false, true, false, false)));
        eq("reveil: meteo disparue", R, Nav.wake(W, new Nav.Flags(true, false, false, false)));
        eq("reveil depuis ambiance", R, Nav.wake(Am, new Nav.Flags(true, true, false, false)));
        // invariants : Retour ramene toujours au repos, jamais d'etat indisponible
        for (Nav.State s : Nav.State.values())
            for (int m = 0; m < 16; m++) {
                boolean fi = (m & 1) != 0, me = (m & 2) != 0, ed = (m & 4) != 0, fr = (m & 8) != 0;
                eq("retour " + s + " " + m, R, go(s, B, fi, me, ed, fr));
                for (Nav.Key k : Nav.Key.values()) {
                    if (k == Nav.Key.IDLE) continue;
                    Nav.State n = go(s, k, fi, me, ed, fr);
                    ok("etat disponible " + s + " " + k + " " + m, Nav.available(n, new Nav.Flags(fi, me, ed, fr)) == n);
                }
            }
    }

    // ---------- Bande-annonce
    static MovieRules.Video vid(String key, String type, String site, boolean official) { return new MovieRules.Video(key, type, site, official); }

    static void trailer() {
        eq("cle valide", "dQw4w9WgXcQ", MovieRules.youtubeKey("dQw4w9WgXcQ"));
        eq("cle avec _ et -", "a_b-C1d2E3f", MovieRules.youtubeKey("a_b-C1d2E3f"));
        eq("cle trop courte", "", MovieRules.youtubeKey("dQw4w9WgXc"));
        eq("cle trop longue", "", MovieRules.youtubeKey("dQw4w9WgXcQx"));
        eq("cle avec &", "", MovieRules.youtubeKey("dQw4w9&gXcQ"));
        eq("cle avec /", "", MovieRules.youtubeKey("dQw4w9/gXcQ"));
        eq("cle avec saut de ligne", "", MovieRules.youtubeKey("dQw4w9WgXc\n"));
        eq("cle avec espace", "", MovieRules.youtubeKey("dQw4w9WgXc "));
        eq("cle vide", "", MovieRules.youtubeKey(""));
        eq("cle null", "", MovieRules.youtubeKey(null));
        List<MovieRules.Video> l = new ArrayList<>();
        eq("aucune video", "", MovieRules.pickTrailer(l));
        l.add(vid("aaaaaaaaaaa", "Teaser", "YouTube", true));
        l.add(vid("bbbbbbbbbbb", "Trailer", "Vimeo", true));
        l.add(vid("bad", "Trailer", "YouTube", true));
        eq("ni Trailer YouTube valide", "", MovieRules.pickTrailer(l));
        l.add(vid("ccccccccccc", "Trailer", "YouTube", false));
        eq("non officielle faute de mieux", "ccccccccccc", MovieRules.pickTrailer(l));
        l.add(vid("ddddddddddd", "Trailer", "YouTube", true));
        eq("officielle en priorite", "ddddddddddd", MovieRules.pickTrailer(l));
        l.add(vid("eeeeeeeeeee", "Trailer", "YouTube", true));
        eq("premiere officielle", "ddddddddddd", MovieRules.pickTrailer(l));
        eq("videosUrl fr", "https://api.themoviedb.org/3/movie/603/videos?language=fr-FR", MovieRules.videosUrl(603, "fr-FR"));
        ok("videosUrl hote autorise", allowed(MovieRules.videosUrl(603, "en-US")));
        eq("watchUrl", "https://www.youtube.com/watch?v=dQw4w9WgXcQ", MovieRules.watchUrl("dQw4w9WgXcQ"));
        ok("backdrop 1280 hote autorise", allowed(MovieRules.backdropFullUrl("/a.jpg")));
        Movie m = mk("/p.jpg", "2026-01-01", "T"); m.id = 42;
        List<Movie> in = new ArrayList<>(); in.add(m);
        eq("select conserve l'id", 42, MovieRules.select(in, false, "2026-01-01").get(0).id);
    }

    // ---------- Durcissement des donnees distantes
    static void hardening() {
        eq("tabStep gauche", 1, MovieRules.tabStep(2, -1)); eq("tabStep droite", 3, MovieRules.tabStep(2, 1));
        eq("tabStep borne bas", 0, MovieRules.tabStep(0, -1)); eq("tabStep borne haut", 3, MovieRules.tabStep(3, 1));
        eq("tabStep hors bornes", -1, MovieRules.tabStep(-1, 1)); eq("tabStep 4 onglets", 3, MovieRules.tabStep(2, 1));
        StringBuilder t = new StringBuilder(), o = new StringBuilder();
        for (int i = 0; i < 500; i++) t.append('t');
        for (int i = 0; i < 3000; i++) o.append('o');
        Movie m = mk("/p.jpg", "2026-01-01", t.toString()); m.overview = o.toString();
        List<Movie> in = new ArrayList<>(); in.add(m);
        Movie r = MovieRules.select(in, false, "2026-01-01").get(0);
        eq("titre tronque", 200, r.title.length()); eq("resume tronque", 1000, r.overview.length());
        Movie s = mk("/p.jpg", "2026-01-01", "court"); s.overview = "ok";
        in.clear(); in.add(s);
        eq("texte court inchange", "court", MovieRules.select(in, false, "2026-01-01").get(0).title);
        eq("cut null", "", MovieRules.cut(null, 5));
        ok("image 2000x1500 ok", !MovieRules.tooBig(2000, 1500)); ok("image 2000x2000 ok", !MovieRules.tooBig(2000, 2000));
        ok("image 2001x2000 refusee", MovieRules.tooBig(2001, 2000)); ok("image 30000x30000 refusee", MovieRules.tooBig(30000, 30000));
        ok("image illisible refusee", MovieRules.tooBig(0, 0));
    }

    // ---------- Ma liste
    static Movie mid(int id, String title) { Movie m = mk("/" + title + ".jpg", "2026-01-01", title); m.id = id; return m; }

    static void myList() {
        eq("tab -1", 0, MovieRules.tab(-1)); eq("tab 0", 0, MovieRules.tab(0)); eq("tab 3", 3, MovieRules.tab(3));
        eq("tab 4", 3, MovieRules.tab(4)); eq("tab 99", 3, MovieRules.tab(99));
        eq("jsonKey 3", "my_list", MovieRules.jsonKey(3)); eq("tsKey 3", "ml_ts", MovieRules.tsKey(3));
        List<Movie> l = new ArrayList<>();
        l = MovieRules.listAdd(l, mid(1, "a"), 3);
        l = MovieRules.listAdd(l, mid(2, "b"), 3);
        eq("ajout en tete", "b,a,", titles(l));
        eq("doublon ignore", "b,a,", titles(MovieRules.listAdd(l, mid(1, "a"), 3)));
        l = MovieRules.listAdd(l, mid(3, "c"), 3);
        eq("plein", null, MovieRules.listAdd(l, mid(4, "d"), 3));
        eq("doublon meme plein", "c,b,a,", titles(MovieRules.listAdd(l, mid(2, "b"), 3)));
        eq("retrait", "c,a,", titles(MovieRules.listRemove(l, mid(2, "b"))));
        eq("retrait absent", "c,b,a,", titles(MovieRules.listRemove(l, mid(9, "z"))));
        eq("index", 1, MovieRules.listIndex(l, mid(2, "b")));
        eq("index absent", -1, MovieRules.listIndex(l, mid(9, "z")));
        List<Movie> noId = new ArrayList<>();
        noId.add(mid(0, "x"));
        eq("sans id : meme titre+affiche", 0, MovieRules.listIndex(noId, mid(0, "x")));
        eq("sans id : autre titre", -1, MovieRules.listIndex(noId, mid(0, "y")));
        eq("liste vide : plafond 30", 30, MovieRules.MY_MAX);
        eq("badge aujourd'hui", 1, MovieRules.releaseBadge("2026-10-25", "2026-10-25"));
        eq("badge sorti", 2, MovieRules.releaseBadge("2026-10-24", "2026-10-25"));
        eq("badge a venir", 0, MovieRules.releaseBadge("2026-10-26", "2026-10-25"));
        eq("badge date vide", 0, MovieRules.releaseBadge("", "2026-10-25"));
        eq("badge date partielle", 0, MovieRules.releaseBadge("2026", "2026-10-25"));
    }

    // ---------- DockMetrics
    static void dock() {
        // n: bigW, bigH, bigM, hint, nxBase, appsShift
        int[][] t = {
            {2, 176, 99, 9, 200, 195, 68}, {3, 176, 99, 9, 200, 195, 68}, {4, 176, 99, 9, 200, 195, 68},
            {5, 176, 99, 9, 200, 195, 68}, {6, 170, 96, 9, 197, 192, 68}, {7, 146, 82, 7, 183, 178, 68},
            {8, 128, 72, 6, 173, 168, 68}, {9, 113, 64, 6, 165, 160, 68}, {10, 102, 57, 5, 158, 153, 68}};
        for (int[] r : t) {
            int n = r[0];
            eq("bigW " + n, r[1], DockMetrics.bigW(n));
            eq("bigH " + n, r[2], DockMetrics.bigH(n));
            eq("bigM " + n, r[3], DockMetrics.bigM(n));
            eq("layoutHint " + n, r[4], DockMetrics.layoutHint(n));
            eq("nxBase " + n, r[5], DockMetrics.nxBase(n));
            eq("appsShift " + n, r[6], DockMetrics.appsShift());
            int width = n * (DockMetrics.bigW(n) + 2 * DockMetrics.bigM(n)) + 2 * DockMetrics.DOCK_PAD;
            ok("largeur dock " + n + " <= 1280 - 2 x 56 (" + width + ")", width <= 1280 - 2 * 56);
            eq("compact " + n + " (seulement N=10)", n == 10, DockMetrics.compact(n));
            // positions verticales (depuis le bas) : rien ne se chevauche, rien ne sort de l'ecran
            int dockTop = DockMetrics.DOCK_BOTTOM + DockMetrics.dockH(n);
            int carTop = DockMetrics.carBase() + DockMetrics.CAR_H;
            ok("repos " + n + ": le carrousel est sous le dock", carTop <= DockMetrics.DOCK_BOTTOM);
            ok("applis " + n + ": le carrousel entier est a l'ecran", DockMetrics.carBase() + DockMetrics.appsShift() >= DockMetrics.CAR_MARGIN);
            ok("applis " + n + ": le dock monte sans toucher la zone du haut",
                    dockTop + DockMetrics.appsShift() + (DockMetrics.layoutHint(n) - dockTop) + 40 <= DockMetrics.SCREEN_H - DockMetrics.TOP_ZONE);
            ok("films " + n + ": au-dessus du dock", DockMetrics.nxBase(n) >= dockTop);
            ok("films " + n + ": sous la zone du haut", DockMetrics.nxBase(n) + DockMetrics.FILMS_H <= DockMetrics.SCREEN_H - DockMetrics.TOP_ZONE);
        }
    }

    // ---------- MovieRules
    static Movie mk(String poster, String date, String title) {
        Movie m = new Movie();
        m.poster = poster; m.date = date; m.title = title;
        return m;
    }

    static String titles(List<Movie> l) {
        StringBuilder b = new StringBuilder();
        for (Movie m : l) b.append(m.title).append(',');
        return b.toString();
    }

    static void movieRules() {
        eq("imgPath jpg", "/abc.jpg", MovieRules.imgPath("/abc.jpg"));
        eq("imgPath png _-", "/a_b-C1.png", MovieRules.imgPath("/a_b-C1.png"));
        eq("imgPath ../x.jpg", "", MovieRules.imgPath("../x.jpg"));
        eq("imgPath /a/b.jpg", "", MovieRules.imgPath("/a/b.jpg"));
        eq("imgPath .jpeg", "", MovieRules.imgPath("/x.jpeg"));
        eq("imgPath ?q", "", MovieRules.imgPath("/x.jpg?q=1"));
        eq("imgPath vide", "", MovieRules.imgPath(""));
        eq("imgPath sans slash", "", MovieRules.imgPath("x.jpg"));
        eq("imgPath \\n final", "", MovieRules.imgPath("/x.jpg\n"));
        eq("imgPath //hote", "", MovieRules.imgPath("//evil.com/x.jpg"));
        eq("imgPath %2e", "", MovieRules.imgPath("/%2e%2e.jpg"));
        eq("imgPath .JPG", "", MovieRules.imgPath("/x.JPG"));
        eq("imgPath chiffre non ASCII", "", MovieRules.imgPath("/\u0664\u0663.jpg"));

        // select
        List<Movie> in = new ArrayList<>();
        in.add(mk("/a.jpg", "2026-12-01", "tard"));
        in.add(mk("", "2026-11-01", "sans-affiche"));
        in.add(mk("/b.jpg", "", "sans-date"));
        in.add(mk("/c.jpg", "2026-01-01", "passe"));
        in.add(mk("/d.jpg", "2026-10-25", "aujourdhui"));
        in.add(mk("/e.jpg", "2026-11-01", "meme-jour-1"));
        in.add(mk("../f.jpg", "2026-11-01", "mauvais-chemin"));
        in.add(mk("/g.jpg", "2026-11-01", "meme-jour-2"));
        eq("select upcoming: filtre + tri stable", "aujourdhui,meme-jour-1,meme-jour-2,tard,",
                titles(MovieRules.select(in, true, "2026-10-25")));
        eq("select non-upcoming: ordre garde, dates ignorees", "tard,sans-date,passe,aujourdhui,meme-jour-1,meme-jour-2,",
                titles(MovieRules.select(in, false, "2026-10-25")));
        List<Movie> many = new ArrayList<>();
        for (int i = 0; i < 20; i++) many.add(mk("/p" + i + ".jpg", "2027-01-01", "m" + i));
        eq("select plafond 12 (upcoming)", 12, MovieRules.select(many, true, "2026-10-25").size());
        eq("select plafond 12 (autre)", 12, MovieRules.select(many, false, "2026-10-25").size());
        eq("select plafond: ordre", "m0,m1,m2,m3,m4,m5,m6,m7,m8,m9,m10,m11,", titles(MovieRules.select(many, true, "2026-10-25")));
        Movie full = mk("/a.jpg", "2026-12-01", "t"); full.overview = "o"; full.backdrop = "/bd.jpg"; full.vote = 7.5;
        List<Movie> one = new ArrayList<>(); one.add(full);
        Movie s = MovieRules.select(one, false, "2026-10-25").get(0);
        ok("select copie les champs", s != full && "t".equals(s.title) && "o".equals(s.overview) && "/a.jpg".equals(s.poster)
                && "/bd.jpg".equals(s.backdrop) && "2026-12-01".equals(s.date) && s.vote == 7.5);
        full.backdrop = "../bad.jpg";
        eq("select assainit le backdrop", "", MovieRules.select(one, false, "2026-10-25").get(0).backdrop);

        // cles de cache, affiches
        eq("jsonKey 0", "nx_json", MovieRules.jsonKey(0));
        eq("jsonKey 1", "nc_json", MovieRules.jsonKey(1));
        eq("jsonKey 2", "nu_json", MovieRules.jsonKey(2));
        eq("tsKey 0", "nx_ts", MovieRules.tsKey(0));
        eq("tsKey 1", "nc_ts", MovieRules.tsKey(1));
        eq("tsKey 2", "nu_ts", MovieRules.tsKey(2));
        eq("posterName fige", POSTER_ABC, MovieRules.posterName("/abc.jpg"));
        eq("posterName stable", MovieRules.posterName("/abc.jpg"), MovieRules.posterName("/abc.jpg"));
        ok("posterName sans /", !MovieRules.posterName("../../x").contains("/") && MovieRules.posterName("../../x").endsWith(".img"));
        ok("posterName distinct", !MovieRules.posterName("/abc.jpg").equals(MovieRules.posterName("/abd.jpg")));
        eq("sample 3000/185", 16, MovieRules.sample(3000, 185));
        eq("sample 185/185", 1, MovieRules.sample(185, 185));
        eq("sample 370/185", 2, MovieRules.sample(370, 185));
        eq("sample 369/185", 1, MovieRules.sample(369, 185));
        eq("sample 0", 1, MovieRules.sample(0, 185));
        eq("sample 2560/1280", 2, MovieRules.sample(2560, 1280));
        eq("posterUrl", "https://image.tmdb.org/t/p/w185/abc.jpg", MovieRules.posterUrl("/abc.jpg"));
        eq("backdropUrl", "https://image.tmdb.org/t/p/w780/abc.jpg", MovieRules.backdropUrl("/abc.jpg"));
        eq("listUrl 0", "https://api.themoviedb.org/3/discover/movie?watch_region=FR&with_watch_providers=8"
                + "&with_watch_monetization_types=flatrate&language=fr-FR&sort_by=primary_release_date.desc"
                + "&primary_release_date.lte=2026-10-25&vote_count.gte=20&page=1", MovieRules.listUrl(0, "2026-10-25"));
        eq("listUrl 1", "https://api.themoviedb.org/3/movie/now_playing?language=fr-FR&region=FR&page=1", MovieRules.listUrl(1, "2026-10-25"));
        eq("listUrl 2", "https://api.themoviedb.org/3/movie/upcoming?language=fr-FR&region=FR&page=1", MovieRules.listUrl(2, "2026-10-25"));
        for (int i = 0; i < 3; i++) ok("listUrl hote autorise " + i, allowed(MovieRules.listUrl(i, "2026-10-25")));
        ok("posterUrl hote autorise", allowed(MovieRules.posterUrl("/abc.jpg")));
        ok("backdropUrl hote autorise", allowed(MovieRules.backdropUrl("/abc.jpg")));

        // lignes meta
        Movie m = mk("/a.jpg", "2026-12-25", "t"); m.vote = 7.46;
        eq("spotMeta onglet 0 fr", "2026  ·  ★ 7,5 / 10", MovieRules.spotMeta(m, 0, true));
        eq("spotMeta onglet 1 en", "2026  ·  ★ 7.5 / 10", MovieRules.spotMeta(m, 1, false));
        eq("spotMeta onglet 2 fr", "Sortie le 25 décembre  ·  ★ 7,5 / 10", MovieRules.spotMeta(m, 2, true));
        eq("spotMeta onglet 2 en", "Releases 25 December  ·  ★ 7.5 / 10", MovieRules.spotMeta(m, 2, false));
        eq("detailMeta onglet 0 fr", "2026  ·  ★ 7,5 / 10", MovieRules.detailMeta(m, 0, true));
        eq("detailMeta onglet 2 fr", "Sortie le 25 décembre 2026  ·  ★ 7,5 / 10", MovieRules.detailMeta(m, 2, true));
        eq("detailMeta onglet 2 en", "Releases 25 December 2026  ·  ★ 7.5 / 10", MovieRules.detailMeta(m, 2, false));
        m.vote = 0;
        eq("spotMeta sans note", "2026", MovieRules.spotMeta(m, 0, true));
        eq("detailMeta sans note", "Sortie le 25 décembre 2026", MovieRules.detailMeta(m, 2, true));
        Movie nd = mk("/a.jpg", "", "t");
        eq("spotMeta onglet 2 sans date", "", MovieRules.spotMeta(nd, 2, true));
        eq("detailMeta onglet 2 sans date", "", MovieRules.detailMeta(nd, 2, true));
        nd.vote = 8;
        eq("meta sans date avec note", "★ 8,0 / 10", MovieRules.detailMeta(nd, 0, true));
        Movie bad = mk("/a.jpg", "2026-1x", "t");
        eq("spotMeta date illisible onglet 2", "", MovieRules.spotMeta(bad, 2, true));
        eq("detailMeta date illisible onglet 2 : annee", "2026", MovieRules.detailMeta(bad, 2, true));
    }

    static final String POSTER_ABC = Integer.toHexString("/abc.jpg".hashCode()) + ".img";

    // ---------- ConfigRules
    static void configRules() {
        eq("jeton ok", "abcdefgh", ConfigRules.token("abcdefgh"));
        eq("jeton rogne", "abcdefgh", ConfigRules.token("  abcdefgh \n"));
        eq("jeton charset", "a.b_c~d+e/f=g-h", ConfigRules.token("a.b_c~d+e/f=g-h"));
        eq("jeton null", null, ConfigRules.token(null));
        eq("jeton 7 car.", null, ConfigRules.token("abcdefg"));
        eq("jeton \\n interne", null, ConfigRules.token("abcd\nefgh"));
        eq("jeton espace interne", null, ConfigRules.token("abcd efgh"));
        eq("jeton 4096 car.", rep('a', 4096), ConfigRules.token(rep('a', 4096)));
        eq("jeton 4097 car.", null, ConfigRules.token(rep('a', 4097)));
        eq("jeton caractere interdit", null, ConfigRules.token("abcdefg;h"));
        eq("jeton \\r interne", null, ConfigRules.token("abcd\refgh"));
        eq("jeton que des blancs", null, ConfigRules.token(" \t\r\n  "));
        eq("jeton non ASCII", null, ConfigRules.token("abcd\u00e9fgh"));

        ok("lat 90", ConfigRules.valid("90", 90));
        ok("lat 90.000000001", !ConfigRules.valid("90.000000001", 90));
        ok("lat 90.1", !ConfigRules.valid("90.1", 90));
        ok("lat -90", ConfigRules.valid("-90", 90));
        ok("lon -180", ConfigRules.valid("-180", 180));
        ok("lat -180", !ConfigRules.valid("-180", 90));
        ok("lon 180.5", !ConfigRules.valid("180.5", 180));
        ok("1e3", !ConfigRules.valid("1e3", 180));
        ok("1000", !ConfigRules.valid("1000", 180));
        ok("43,6", !ConfigRules.valid("43,6", 90));
        ok("48.86", ConfigRules.valid("48.86", 90));
        ok("43.12345678 (8 decimales)", ConfigRules.valid("43.12345678", 90));
        ok(" 43.6 rogne", ConfigRules.valid(" 43.6 ", 90));
        ok("vide", !ConfigRules.valid("", 90));
        ok("espaces", !ConfigRules.valid("   ", 90));
        ok("null", !ConfigRules.valid(null, 90));
        ok("+43", !ConfigRules.valid("+43", 90));
        ok("valid NaN", !ConfigRules.valid("NaN", 90));
        ok("valid Infinity", !ConfigRules.valid("Infinity", 90));
        ok("valid chiffres arabes", !ConfigRules.valid("\u0664\u0663", 90));
        double[] c = ConfigRules.coords("48.86", "2.35");
        ok("coords ok", c != null && c[0] == 48.86 && c[1] == 2.35);
        ok("coords bornes", ConfigRules.coords("90", "180") != null && ConfigRules.coords("-90", "-180") != null);
        ok("coords lat hors bornes", ConfigRules.coords("91", "0") == null);
        ok("coords lon hors bornes", ConfigRules.coords("0", "181") == null);
        ok("coords vides", ConfigRules.coords("", "") == null);
        ok("coords illisibles", ConfigRules.coords("abc", "1") == null);
        ok("coords NaN", ConfigRules.coords("NaN", "1") == null && ConfigRules.coords("1", "NaN") == null);
        ok("coords Infinity", ConfigRules.coords("Infinity", "1") == null && ConfigRules.coords("1", "-Infinity") == null);
        ok("coords chiffres arabes", ConfigRules.coords("\u0664\u0663", "1") == null && ConfigRules.coords("1", "\u0664\u0663") == null);

        eq("ville simple", "Paris", ConfigRules.city("Paris"));
        eq("ville rognee", "Paris", ConfigRules.city("  Paris  "));
        eq("ville controle", "Paris", ConfigRules.city("Pa\u0007ri\ns"));
        eq("ville accents", "Saint-Étienne", ConfigRules.city("Saint-Étienne"));
        eq("ville 48", rep('a', 48), ConfigRules.city(rep('a', 48)));
        eq("ville 49 -> 48", rep('a', 48), ConfigRules.city(rep('a', 49)));
        eq("ville vide", "", ConfigRules.city(""));
        eq("ville que des controles", "", ConfigRules.city("\t\n\u0001 "));

        eq("csv separateurs", "[a, b, c, d]", ConfigRules.csv("a,b;c d").toString());
        eq("csv vides ignores", "[a, b]", ConfigRules.csv(",a,,b,").toString());
        eq("csv blanc", "[]", ConfigRules.csv("  ").toString());
        eq("csv doublons gardes", "[a, a]", ConfigRules.csv("a,a").toString());

        eq("layout 2", 2, ConfigRules.layout("2"));
        eq("layout 10", 10, ConfigRules.layout("10"));
        eq("layout 6", 6, ConfigRules.layout("6"));
        eq("layout 1 -> 4", 4, ConfigRules.layout("1"));
        eq("layout 11 -> 4", 4, ConfigRules.layout("11"));
        eq("layout -3 -> 4", 4, ConfigRules.layout("-3"));
        eq("layout texte -> 4", 4, ConfigRules.layout("x"));
        eq("layout vide -> 4", 4, ConfigRules.layout(""));
        eq("layout espace -> 4", 4, ConfigRules.layout(" 5"));
        // dock affiche vs liste recalculee au retour sur l'accueil (appli desinstallee, desactivee ou reactivee entre-temps)
        List<String> dk = java.util.Arrays.asList("com.netflix.ninja", "com.plexapp.android", "com.spotify.tv.android");
        List<String> sansPlex = java.util.Arrays.asList("com.netflix.ninja", "com.spotify.tv.android");
        ok("dock: appli desinstallee -> reconstruire", ConfigRules.dockStale(dk, sansPlex));
        ok("dock: appli reactivee -> reconstruire", ConfigRules.dockStale(sansPlex, dk));
        ok("dock: rien de change -> rien", !ConfigRules.dockStale(dk, new ArrayList<>(dk)));
        ok("dock vide -> rien", !ConfigRules.dockStale(new ArrayList<String>(), new ArrayList<String>()));
    }

    // ---------- Entrees TV
    static InputRules.Entry in(String id, String custom, String loaded, int type, int state) {
        return InputRules.entry(id, custom, loaded, type, state, true);
    }

    static String ids(java.util.List<InputRules.Entry> l) {
        StringBuilder b = new StringBuilder();
        for (InputRules.Entry e : l) b.append(e.id.substring(e.id.lastIndexOf('/') + 1)).append(' ');
        return b.toString().trim();
    }

    static void inputs() {
        String h = "com.tcl.tvinput/.TvPassThroughService/HW";
        java.util.List<InputRules.Entry> l = new ArrayList<>();
        l.add(in("com.tcl.tvinput/.TunerInputService/HW2", null, "TV", 0, 0));
        l.add(in(h + "18", null, "HDMI 4", 1007, 0));
        l.add(in(h + "16", null, "HDMI 2", 1007, 2));
        l.add(in(h + "15", null, "HDMI 1", 1007, 1));
        l.add(in(h + "17", null, "HDMI 3", 1007, 0));
        l.add(in(h + "3", null, "AV", 1001, 2));
        eq("tri: branches, veille, debranches ; HDMI avant AV avant tuner", "HW17 HW18 HW2 HW15 HW16 HW3", ids(InputRules.sort(l)));
        eq("tri: l'entree d'origine n'est pas modifiee", "HW2", ids(l.subList(0, 1)));
        eq("tri: HW9 < HW15 (numerique)", "HW9 HW15", ids(InputRules.sort(java.util.Arrays.asList(in(h + "15", "", "a", 1007, 0), in(h + "9", "", "b", 1007, 0)))));
        eq("tri: vide", "", ids(InputRules.sort(new ArrayList<InputRules.Entry>())));

        eq("libelle perso prioritaire", "PlayStation", InputRules.label("PlayStation", "HDMI 2", 1007, true));
        eq("libelle perso rogne", "Box", InputRules.label("  Box ", "HDMI 2", 1007, true));
        eq("libelle perso vide -> systeme", "HDMI 2", InputRules.label("   ", "HDMI 2", 1007, true));
        eq("libelle perso null -> systeme", "HDMI 2", InputRules.label(null, "HDMI 2", 1007, true));
        eq("libelle repli hdmi", "HDMI", InputRules.label(null, null, 1007, true));
        eq("libelle repli tuner fr", "Antenne / TV", InputRules.label("", "", 0, true));
        eq("libelle repli tuner en", "Tuner / TV", InputRules.label("", "", 0, false));
        eq("libelle repli av", "Entrée AV", InputRules.label("", "", 1001, true));

        eq("etat 0 fr", "Branché", InputRules.stateText(0, true)); eq("etat 1 fr", "En veille", InputRules.stateText(1, true));
        eq("etat 2 fr", "Non branché", InputRules.stateText(2, true)); eq("etat 2 en", "Not connected", InputRules.stateText(2, false));
        eq("etat 0 en", "Connected", InputRules.stateText(0, false)); eq("etat 1 en", "Standby", InputRules.stateText(1, false));
        eq("etat inconnu", "", InputRules.stateText(9, true));

        eq("numero HW16", 16, InputRules.number(h + "16")); eq("numero sans chiffre", -1, InputRules.number("a/b"));
        eq("numero null", -1, InputRules.number(null)); eq("numero vide", -1, InputRules.number("")); eq("numero 9 chiffres", -1, InputRules.number("x/123456789"));

        eq("nom hdmi", "plug", InputRules.iconName(1007, "HDMI 1")); eq("nom hdmi null", "plug", InputRules.iconName(1007, null));
        eq("nom composite", "plugs", InputRules.iconName(1001, "AV")); eq("nom composante", "plugs", InputRules.iconName(1004, "Component"));
        eq("nom tuner", "television-simple", InputRules.iconName(0, "TV")); eq("nom vga", "television-simple", InputRules.iconName(1005, "PC"));
        eq("nom barre de son", "speaker-hifi", InputRules.iconName(1007, "Salon SB300")); eq("nom soundbar", "speaker-hifi", InputRules.iconName(1007, "Samsung Soundbar"));
        eq("nom barre fr", "speaker-hifi", InputRules.iconName(1007, "Barre de son salon")); eq("nom avr", "speaker-hifi", InputRules.iconName(1007, "Denon AVR-X"));
        eq("nom lecteur", "plug", InputRules.iconName(1007, "Lecteur")); eq("nom avril n'est pas un avr", "plug", InputRules.iconName(1007, "Avril"));
        eq("nom enceinte sur AV", "plugs", InputRules.iconName(1001, "Enceinte"));
        eq("pastille branche", 0xFF4CD964, InputRules.stateColor(0)); eq("pastille veille", 0xFFFFB020, InputRules.stateColor(1));
        eq("pastille debranche", 0x80FFFFFF, InputRules.stateColor(2)); eq("pastille inconnue", 0x80FFFFFF, InputRules.stateColor(9));
        ok("alpha debranche", InputRules.cardAlpha(2) == 0.5f); ok("alpha branche", InputRules.cardAlpha(0) == 1f); ok("alpha veille", InputRules.cardAlpha(1) == 1f);
        eq("cartes: entrees + Plus", 7, InputRules.cardCount(6));
        eq("carte 4 cartes = tuile du dock 4", 176, InputRules.cardW(4)); eq("carte 4 hauteur", 99, InputRules.cardH(4));
        eq("carte 7 (6 entrees) = dock 7", DockMetrics.bigW(7), InputRules.cardW(7));
        ok("6 entrees tiennent en largeur", InputRules.cardCount(6) * (InputRules.cardW(7) + 2 * InputRules.cardM(7)) + 40 <= 1280 - 112);
        ok("9 entrees tiennent en largeur", InputRules.cardCount(9) * (InputRules.cardW(10) + 2 * InputRules.cardM(10)) + 40 <= 1280 - 112);
        eq("carte 1 -> comme 2", InputRules.cardW(2), InputRules.cardW(1)); eq("carte 8 plus petite que 7", true, InputRules.cardW(8) < InputRules.cardW(7));
        java.util.List<InputRules.Entry> fl = new ArrayList<>();
        fl.add(in("x/HW1", null, "a", 1007, 2)); fl.add(in("x/HW2", null, "b", 1007, 1)); fl.add(in("x/HW3", null, "c", 1007, 0));
        eq("focus: premiere branchee", 2, InputRules.focusIndex(fl));
        fl.remove(2); eq("focus: aucune branchee -> 0", 0, InputRules.focusIndex(fl));
        eq("focus: liste vide -> 0", 0, InputRules.focusIndex(new ArrayList<InputRules.Entry>()));

        ok("garde: entree materielle", InputRules.keep(true, 1007, false)); ok("garde: tuner du meme paquet", InputRules.keep(false, 0, true));
        ok("ecarte: appli de chaines", !InputRules.keep(false, 0, false)); ok("ecarte: autre type non materiel", !InputRules.keep(false, 1000, true));
    }

    // ---------- Minuteur de veille
    static void sleep() {
        long now = 1_000_000_000L, min = 60_000L;
        eq("echeance 30 min", now + 30 * min, SleepRules.deadline(now, 30));
        eq("echeance desactive", 0L, SleepRules.deadline(now, 0));
        eq("echeance negative", 0L, SleepRules.deadline(now, -5));
        ok("actif: futur", SleepRules.active(now + 1, now)); ok("inactif: passe", !SleepRules.active(now - 1, now));
        ok("inactif: pile maintenant", !SleepRules.active(now, now)); ok("inactif: 0", !SleepRules.active(0, now));
        eq("restant 42 min", 42, SleepRules.minutesLeft(now + 42 * min, now));
        eq("restant arrondi au-dessus", 42, SleepRules.minutesLeft(now + 41 * min + 30_000, now));
        eq("restant 1 s -> 1", 1, SleepRules.minutesLeft(now + 1000, now));
        eq("restant passe", 0, SleepRules.minutesLeft(now - 5, now));
        eq("restant sans minuteur", 0, SleepRules.minutesLeft(0, now));
        eq("texte fr", "Veille dans 42 min", SleepRules.leftText(now + 42 * min, now, true));
        eq("texte en", "Sleep in 42 min", SleepRules.leftText(now + 42 * min, now, false));
        eq("texte inactif", "", SleepRules.leftText(now - 1, now, true));
        eq("avertissement 30 min", now + 29 * min, SleepRules.warnAt(now + 30 * min, now));
        eq("avertissement trop tard", 0L, SleepRules.warnAt(now + 60_000L, now));
        eq("avertissement 61 s", now + 1000, SleepRules.warnAt(now + 61_000L, now));
        eq("avertissement sans minuteur", 0L, SleepRules.warnAt(0, now));
        eq("choix off fr", "Désactivé", SleepRules.choiceLabel(0, true)); eq("choix off en", "Off", SleepRules.choiceLabel(0, false));
        eq("choix 90", "90 min", SleepRules.choiceLabel(90, true));
        eq("choix 8 valeurs", "[0, 15, 30, 60, 90, 120, 180, 240]", java.util.Arrays.toString(SleepRules.CHOICES));
        eq("choix 120", "120 min", SleepRules.choiceLabel(120, true));
        eq("choix 180", "180 min", SleepRules.choiceLabel(180, true)); eq("choix 240", "240 min", SleepRules.choiceLabel(240, false));
        eq("choix 15", "15 min", SleepRules.choiceLabel(15, true));
        eq("index 240", 7, SleepRules.indexOf(240));
        eq("index 90", 4, SleepRules.indexOf(90)); eq("index inconnu -> 0", 0, SleepRules.indexOf(7));
        ok("due: a l'heure", SleepRules.due(now, now, now)); ok("due: 3 s avant", SleepRules.due(now, now, now - 3000));
        ok("pas due: 10 s avant", !SleepRules.due(now, now, now - 10_000)); ok("due: en retard", SleepRules.due(now, now, now + 90_000));
        ok("pas due: annule (prefs a 0)", !SleepRules.due(0, now, now)); ok("pas due: echeance remplacee", !SleepRules.due(now + min, now, now));
        ok("pas due: extra absent", !SleepRules.due(now, -1, now));
        ok("avert: ok", SleepRules.warnDue(now, now, now - min)); ok("avert: apres l'echeance", !SleepRules.warnDue(now, now, now));
        ok("avert: annule", !SleepRules.warnDue(0, now, now - min)); ok("avert: remplace", !SleepRules.warnDue(now + min, now, now - min));
        eq("court: 42 min", "42 min", SleepRules.shortLeft(now + 42 * min, now));
        eq("court: 60 min", "60 min", SleepRules.shortLeft(now + 60 * min, now));
        eq("court: 61 min", "1 h 01", SleepRules.shortLeft(now + 61 * min, now));
        eq("court: 1 h 05", "1 h 05", SleepRules.shortLeft(now + 65 * min, now));
        eq("court: 2 h pile", "2 h 00", SleepRules.shortLeft(now + 120 * min, now));
        eq("court: 3 h 20", "3 h 20", SleepRules.shortLeft(now + 200 * min, now));
        eq("court: 3 h 19 30 s -> 3 h 20", "3 h 20", SleepRules.shortLeft(now + 199 * min + 30_000, now));
        eq("court: 4 h", "4 h 00", SleepRules.shortLeft(now + 240 * min, now));
        eq("court: moins d'une minute", "1 min", SleepRules.shortLeft(now + 5000, now));
        eq("court: inactif", "", SleepRules.shortLeft(now - 1, now)); eq("court: aucun", "", SleepRules.shortLeft(0, now));
        eq("texte fr 3 h 20", "Veille dans 3 h 20", SleepRules.leftText(now + 200 * min, now, true));
        eq("texte en 1 h 05", "Sleep in 1 h 05", SleepRules.leftText(now + 65 * min, now, false));
        eq("texte fr 60 min", "Veille dans 60 min", SleepRules.leftText(now + 60 * min, now, true));
        eq("texte avert fr", "Mise en veille dans 1 minute", SleepRules.warnText(true));
    }

    // ---------- Meteo : heures et pluie
    static final long H = 3_600_000L, Q = 900_000L;

    static void weatherRules() {
        long d0 = WeatherRules.local("2026-10-25T00:00");
        eq("local: epoch", 0L, WeatherRules.local("1970-01-01T00:00"));
        eq("local: 2026-10-25T00:00", 1792886400000L, d0);
        eq("local: +14 h 30", d0 + 14 * H + 30 * 60_000L, WeatherRules.local("2026-10-25T14:30"));
        eq("local: bissextile", 86400000L, WeatherRules.local("2028-03-01T00:00") - WeatherRules.local("2028-02-29T00:00"));
        eq("local: passage d'annee", H, WeatherRules.local("2027-01-01T00:00") - WeatherRules.local("2026-12-31T23:00"));
        eq("local: illisible", -1L, WeatherRules.local("hier")); eq("local: null", -1L, WeatherRules.local(null));
        eq("local: mois 13", -1L, WeatherRules.local("2026-13-01T00:00")); eq("local: heure 24", -1L, WeatherRules.local("2026-10-25T24:00"));
        eq("local: chiffres faux", -1L, WeatherRules.local("2026-1x-25T00:00"));
        eq("now local +2 h", d0 + 2 * H, WeatherRules.nowLocal(d0, 7200));

        long[] hrs = new long[24];
        for (int i = 0; i < 24; i++) hrs[i] = d0 + i * H;
        eq("8 heures: 14:35 -> 15..22", "[15, 16, 17, 18, 19, 20, 21, 22]", idx(WeatherRules.nextHours(hrs, d0 + 14 * H + 35 * 60_000L, 8)));
        eq("8 heures: pile 14:00 -> inclut 14", "[14, 15, 16, 17, 18, 19, 20, 21]", idx(WeatherRules.nextHours(hrs, d0 + 14 * H, 8)));
        eq("8 heures: fin de donnees", "[22, 23]", idx(WeatherRules.nextHours(hrs, d0 + 21 * H + 1, 8)));
        eq("8 heures: tout passe", "[]", idx(WeatherRules.nextHours(hrs, d0 + 30 * H, 8)));
        eq("8 heures: tableau vide", "[]", idx(WeatherRules.nextHours(new long[0], d0, 8)));
        eq("8 heures: avant les donnees", "[0, 1]", idx(WeatherRules.nextHours(hrs, d0 - H, 2)));

        long now = d0 + 14 * H + 35 * 60_000L;   // 14:35
        long[] ht = {d0 + 14 * H, d0 + 15 * H, d0 + 16 * H, d0 + 17 * H};
        long[] qt = {now - Q, now + Q, now + 2 * Q, now + 3 * Q, now + 4 * Q, now + 5 * Q};
        double[] none = {0, 0, 0, 0}, qnone = {0, 0, 0, 0, 0, 0};
        eq("pluie: rien", -1L, WeatherRules.rainStart(ht, none, qt, qnone, now));
        eq("pluie: proba 50 % sur ]14h,15h] -> en cours", now, WeatherRules.rainStart(ht, new double[]{0, 50, 0, 0}, qt, qnone, now));
        eq("pluie: proba 49 % ignoree", -1L, WeatherRules.rainStart(ht, new double[]{0, 49, 0, 0}, qt, qnone, now));
        eq("pluie: proba 80 % sur ]15h,16h] -> debut 15h", d0 + 15 * H, WeatherRules.rainStart(ht, new double[]{0, 0, 80, 0}, qt, qnone, now));
        eq("pluie: proba 80 % sur ]16h,17h] -> hors fenetre (15:35)", -1L, WeatherRules.rainStart(ht, new double[]{0, 0, 0, 80}, qt, qnone, now));
        eq("pluie: proba inconnue ignoree", -1L, WeatherRules.rainStart(ht, new double[]{-1, -1, -1, -1}, qt, qnone, now));
        eq("pluie: 0,2 mm dans 30 min", now + 2 * Q - Q, WeatherRules.rainStart(ht, none, qt, new double[]{0, 0, 0.2, 0, 0, 0}, now));
        eq("pluie: exactement 0,1 mm ignore", -1L, WeatherRules.rainStart(ht, none, qt, new double[]{0, 0.1, 0.1, 0, 0, 0}, now));
        eq("pluie: quart passe ignore", -1L, WeatherRules.rainStart(ht, none, qt, new double[]{5, 0, 0, 0, 0, 0}, now));
        eq("pluie: quart fini a 15:35 + 15 = hors fenetre", -1L, WeatherRules.rainStart(ht, none, new long[]{now + 5 * Q + Q * 3}, new double[]{3}, now));
        eq("pluie: le plus tot gagne", now + 2 * Q - Q, WeatherRules.rainStart(ht, new double[]{0, 0, 90, 0}, qt, new double[]{0, 0, 0.5, 0, 0, 1}, now));
        eq("pluie: sans donnees", -1L, WeatherRules.rainStart(null, null, null, null, now));
        eq("pluie: horaire seul", now, WeatherRules.rainStart(ht, new double[]{0, 60, 0, 0}, null, null, now));
        eq("pluie: minutely seul", now + Q - Q, WeatherRules.rainStart(null, null, qt, new double[]{0, 1, 0, 0, 0, 0}, now));

        eq("texte: rien", "", WeatherRules.rainText(-1, now, true));
        eq("texte fr: en cours", "Pluie dans l'heure", WeatherRules.rainText(now, now, true));
        eq("texte en: en cours", "Rain within the hour", WeatherRules.rainText(now, now, false));
        eq("texte fr: dans 29 min", "Pluie dans l'heure", WeatherRules.rainText(now + 29 * 60_000L, now, true));
        eq("texte fr: 21 h", "Pluie vers 21 h", WeatherRules.rainText(d0 + 21 * H, d0 + 20 * H + 10 * 60_000L, true));
        eq("texte fr: 21 h 15", "Pluie vers 21 h 15", WeatherRules.rainText(d0 + 21 * H + Q, d0 + 20 * H + 30 * 60_000L, true));
        eq("texte en: 21:00", "Rain around 21:00", WeatherRules.rainText(d0 + 21 * H, d0 + 20 * H + 10 * 60_000L, false));
        eq("texte en: 09:05", "Rain around 9:05", WeatherRules.rainText(d0 + 9 * H + 5 * 60_000L, d0 + 8 * H + 20 * 60_000L, false));
        eq("heure fr", "15 h", WeatherRules.hourLabel(d0 + 15 * H, true)); eq("heure en", "15:00", WeatherRules.hourLabel(d0 + 15 * H, false));
        eq("heure 0 fr", "0 h", WeatherRules.hourLabel(d0, true)); eq("heure 9 en", "09:00", WeatherRules.hourLabel(d0 + 9 * H, false));
        eq("pop 60", "60%", WeatherRules.popText(60)); eq("pop 20", "20%", WeatherRules.popText(20)); eq("pop 19", "", WeatherRules.popText(19));
        eq("pop inconnue", "", WeatherRules.popText(-1)); eq("pop NaN", "", WeatherRules.popText(Double.NaN));
    }

    static String idx(int[] a) { return java.util.Arrays.toString(a); }

    // ---------- Touche Source
    static void keyRules() {
        ok("source: TV_INPUT, accueil au premier plan", KeyRules.shouldHandleSource(178, true, true));
        ok("source: TV, accueil au premier plan", KeyRules.shouldHandleSource(170, true, true));
        ok("source: TV_INPUT dans une autre appli", !KeyRules.shouldHandleSource(178, false, true));
        ok("source: TV dans une autre appli", !KeyRules.shouldHandleSource(170, false, true));
        ok("source: autre touche (OK)", !KeyRules.shouldHandleSource(23, true, true));
        ok("source: touche TCL Netflix", !KeyRules.shouldHandleSource(4062, true, true));
        ok("source: code 0", !KeyRules.shouldHandleSource(0, true, true));
        ok("source: aucune entree physique (TV_INPUT)", !KeyRules.shouldHandleSource(178, true, false));
        ok("source: aucune entree physique (TV)", !KeyRules.shouldHandleSource(170, true, false));
        ok("carte entrees: cible et entrees", InputRules.showCard(true, true));
        ok("carte entrees: aucune entree -> masquee", !InputRules.showCard(true, false));
        ok("carte entrees: aucune cible -> masquee", !InputRules.showCard(false, true));
    }

    static void settingsKey() {
        ok("reglages: touche 176", KeyRules.isSettingsKey(176)); ok("reglages: pas 178", !KeyRules.isSettingsKey(178)); ok("reglages: pas 82 (menu)", !KeyRules.isSettingsKey(82));
        eq("reglages: accueil au premier plan -> menu", KeyRules.HOME_MENU, KeyRules.settingsAction(176, true, true));
        eq("reglages: accueil sans panneau TCL -> menu", KeyRules.HOME_MENU, KeyRules.settingsAction(176, true, false));
        eq("reglages: autre appli, TCL -> panneau Image", KeyRules.TCL_PANEL, KeyRules.settingsAction(176, false, true));
        eq("reglages: autre appli, pas TCL -> la touche passe", KeyRules.NONE, KeyRules.settingsAction(176, false, false));
        // telecommande TCL : la touche engrenage envoie KEYCODE_NOTIFICATION (83)
        ok("reglages: touche TCL 83", KeyRules.isSettingsKey(83));
        eq("reglages TCL 83: accueil -> menu", KeyRules.HOME_MENU, KeyRules.settingsAction(83, true, true));
        eq("reglages TCL 83: autre appli -> panneau Image", KeyRules.TCL_PANEL, KeyRules.settingsAction(83, false, true));
        eq("reglages TCL 83: TV non TCL -> la touche passe (meme sur l'accueil)", KeyRules.NONE, KeyRules.settingsAction(83, true, false));
        eq("reglages: autre touche -> rien", KeyRules.NONE, KeyRules.settingsAction(178, true, true));
        eq("reglages: code 0", KeyRules.NONE, KeyRules.settingsAction(0, false, true));
    }

    // ---------- Bandeau « sort aujourd'hui »
    static void banner() {
        List<Movie> l = new ArrayList<>();
        l.add(mk("/a.jpg", "2026-10-25", "Godzilla Minus Zero"));
        l.add(mk("/b.jpg", "2026-11-01", "Plus tard"));
        l.add(mk("/c.jpg", "2026-10-24", "Deja sorti"));
        l.add(mk("/d.jpg", "", "Sans date"));
        l.add(mk("/e.jpg", "2026-10", "Date partielle"));
        eq("sorties: une", "[Godzilla Minus Zero]", MovieRules.releasingToday(l, "2026-10-25").toString());
        l.add(mk("/f.jpg", "2026-10-25", "Autre"));
        eq("sorties: deux, ordre de la liste", "[Godzilla Minus Zero, Autre]", MovieRules.releasingToday(l, "2026-10-25").toString());
        eq("sorties: aucune", "[]", MovieRules.releasingToday(l, "2026-12-25").toString());
        eq("sorties: liste vide", "[]", MovieRules.releasingToday(new ArrayList<Movie>(), "2026-10-25").toString());
        eq("sorties: sans titre ignore", "[]", MovieRules.releasingToday(java.util.Collections.singletonList(mk("/g.jpg", "2026-10-25", "")), "2026-10-25").toString());
        ok("du: premiere fois", MovieRules.bannerDue("", "2026-10-25", 1));
        ok("du: autre jour", MovieRules.bannerDue("2026-10-24", "2026-10-25", 2));
        ok("pas du: deja vu aujourd'hui", !MovieRules.bannerDue("2026-10-25", "2026-10-25", 1));
        ok("pas du: aucune sortie", !MovieRules.bannerDue("", "2026-10-25", 0));
        eq("texte fr 1", "Godzilla Minus Zero sort aujourd'hui", MovieRules.bannerText(java.util.Arrays.asList("Godzilla Minus Zero"), true));
        eq("texte en 1", "Dune is out today", MovieRules.bannerText(java.util.Arrays.asList("Dune"), false));
        eq("texte fr 2", "2 films de ta liste sortent aujourd'hui", MovieRules.bannerText(java.util.Arrays.asList("A", "B"), true));
        eq("texte en 3", "3 movies on your list are out today", MovieRules.bannerText(java.util.Arrays.asList("A", "B", "C"), false));
        eq("texte vide", "", MovieRules.bannerText(new ArrayList<String>(), true));
    }

    // ---------- Jauges RAM / disque
    static void gauge() {
        eq("pct 73", 73, GaugeRules.pct(73, 100)); eq("pct arrondi 72,5 -> 73", 73, GaugeRules.pct(725, 1000));
        eq("pct 0", 0, GaugeRules.pct(0, 100)); eq("pct 100", 100, GaugeRules.pct(100, 100));
        eq("pct RAM reelle", 61, GaugeRules.pct(3921624 - 1520000, 3921624));
        eq("pct total nul", -1, GaugeRules.pct(1, 0)); eq("pct total negatif", -1, GaugeRules.pct(1, -5));
        eq("pct utilise negatif", -1, GaugeRules.pct(-1, 100)); eq("pct utilise > total", -1, GaugeRules.pct(101, 100));
        eq("pct NaN", -1, GaugeRules.pct(Double.NaN, 100)); eq("pct total NaN", -1, GaugeRules.pct(1, Double.NaN));
        eq("pct total infini", -1, GaugeRules.pct(1, Double.POSITIVE_INFINITY));
        eq("texte fr", "73\u202f%", GaugeRules.text(73, true)); eq("texte en", "73%", GaugeRules.text(73, false));
        eq("texte 0 fr", "0\u202f%", GaugeRules.text(0, true)); eq("texte invalide", "", GaugeRules.text(-1, true)); eq("texte > 100", "", GaugeRules.text(101, false));
        ok("arc 0", GaugeRules.sweep(0) == 0f); ok("arc 25 = 90", Math.abs(GaugeRules.sweep(25) - 90f) < 1e-3);
        ok("arc 100 = 360", Math.abs(GaugeRules.sweep(100) - 360f) < 1e-3); ok("arc 73", Math.abs(GaugeRules.sweep(73) - 262.8f) < 1e-3);
        ok("arc invalide = 0", GaugeRules.sweep(-1) == 0f); ok("arc > 100 borne", Math.abs(GaugeRules.sweep(150) - 360f) < 1e-3);
        ok("orange au-dela de 85", GaugeRules.warn(86)); ok("pas orange a 85", !GaugeRules.warn(85)); ok("pas orange invalide", !GaugeRules.warn(-1));
    }

    // ---------- Ecran de veille
    static void standby() {
        eq("meteo fr", "17° ☀ Ensoleillé  ·  10° / 21°", StandbyRules.weatherLine(17.4, 0, 10.2, 20.6, true, "☀", false));
        eq("meteo en", "5° ☁ Overcast  ·  -1° / 7°", StandbyRules.weatherLine(5.2, 3, -1.2, 7.1, false, "☁", false));
        eq("meteo sans min/max", "17° ☀ Ensoleillé", StandbyRules.weatherLine(17.4, 0, Double.NaN, Double.NaN, true, "☀", false));
        eq("meteo min seul manquant", "17° ☀ Ensoleillé", StandbyRules.weatherLine(17.4, 0, Double.NaN, 21, true, "☀", false));
        eq("meteo sans temperature", "", StandbyRules.weatherLine(Double.NaN, 0, 1, 2, true, "☀", false));
        eq("demain fr", "Demain : 19° ☁", StandbyRules.tomorrowLine(18.6, 3, true, "☁"));
        eq("demain en", "Tomorrow: 19° ☁", StandbyRules.tomorrowLine(18.6, 3, false, "☁"));
        eq("demain sans donnee", "", StandbyRules.tomorrowLine(Double.NaN, 3, true, "☁"));
        long now = 1_000_000_000L, h = 3_600_000L;
        ok("frais: 1 h", StandbyRules.fresh(now - h, now)); ok("frais: 3 h pile", StandbyRules.fresh(now - 3 * h, now));
        ok("pas frais: 3 h et 1 ms", !StandbyRules.fresh(now - 3 * h - 1, now)); ok("pas frais: sans date", !StandbyRules.fresh(0, now));
        eq("jours entre", 3L, StandbyRules.daysBetween("2026-10-25", "2026-10-28"));
        eq("jours: fin de mois", 7L, StandbyRules.daysBetween("2026-10-28", "2026-11-04"));
        eq("jours: negatif", -1L, StandbyRules.daysBetween("2026-10-25", "2026-10-24"));
        eq("jours: illisible", Long.MIN_VALUE, StandbyRules.daysBetween("2026-10-25", "bientot"));

        String t = "2026-10-25";
        eq("sortie aujourd'hui fr", "Dune sort aujourd'hui", StandbyRules.releaseText("Dune", t, t, true));
        eq("sortie today en", "Dune is out today", StandbyRules.releaseText("Dune", t, t, false));
        eq("sortie demain", "Godzilla Minus Zero sort demain", StandbyRules.releaseText("Godzilla Minus Zero", "2026-10-26", t, true));
        eq("sortie tomorrow", "Dune is out tomorrow", StandbyRules.releaseText("Dune", "2026-10-26", t, false));
        eq("sortie dans 3 jours", "Dune sort dans 3 jours", StandbyRules.releaseText("Dune", "2026-10-28", t, true));
        eq("sortie in 7 days", "Dune is out in 7 days", StandbyRules.releaseText("Dune", "2026-11-01", t, false));
        eq("sortie 8 jours -> date", "Dune sort le 2 novembre", StandbyRules.releaseText("Dune", "2026-11-02", t, true));
        eq("sortie le 4 novembre", "Dune sort le 4 novembre", StandbyRules.releaseText("Dune", "2026-11-04", t, true));
        eq("sortie le 1er", "Dune sort le 1er décembre", StandbyRules.releaseText("Dune", "2026-12-01", t, true));
        eq("sortie on November 4", "Dune is out on November 4", StandbyRules.releaseText("Dune", "2026-11-04", t, false));
        eq("sortie annee suivante", "Dune sort le 15 janvier", StandbyRules.releaseText("Dune", "2027-01-15", t, true));
        eq("sortie passee", "", StandbyRules.releaseText("Dune", "2026-10-24", t, true));
        eq("sortie date illisible", "", StandbyRules.releaseText("Dune", "demain", t, true));

        List<Movie> l = new ArrayList<>();
        l.add(mk("/a.jpg", "2026-11-04", "Loin")); l.add(mk("/b.jpg", "2026-10-24", "Passe"));
        l.add(mk("/c.jpg", "2026-10-27", "Proche")); l.add(mk("/d.jpg", "2026-10-27", "Proche bis"));
        l.add(mk("/e.jpg", "", "Sans date")); l.add(mk("/f.jpg", "2026-10-26", ""));
        eq("prochaine: la plus proche, 1er en cas d'egalite", "Proche", StandbyRules.nextRelease(l, t).title);
        l.add(mk("/g.jpg", "2026-10-25", "Aujourd'hui"));
        eq("prochaine: aujourd'hui compte", "Aujourd'hui", StandbyRules.nextRelease(l, t).title);
        ok("prochaine: toutes passees", StandbyRules.nextRelease(l, "2027-01-01") == null);
        ok("prochaine: liste vide", StandbyRules.nextRelease(new ArrayList<Movie>(), t) == null);
        ok("prochaine: date mal formee ignoree", StandbyRules.nextRelease(java.util.Collections.singletonList(mk("/h.jpg", "2026-13-45", "X")), t) == null);
    }

    // ---------- Icones Phosphor
    static void iconRules() {
        eq("icone television-simple", 0xEAE6, IconRules.codepoint("television-simple")); eq("icone squares-four", 0xE464, IconRules.codepoint("squares-four"));
        eq("icone gear-six", 0xE272, IconRules.codepoint("gear-six")); eq("icone pencil-simple", 0xE3B4, IconRules.codepoint("pencil-simple"));
        eq("icone image", 0xE2CA, IconRules.codepoint("image")); eq("icone moon", 0xE330, IconRules.codepoint("moon"));
        eq("icone google-play-logo", 0xE294, IconRules.codepoint("google-play-logo"));
        eq("icone inconnue", -1, IconRules.codepoint("house")); eq("icone null", -1, IconRules.codepoint(null)); eq("icone vide", -1, IconRules.codepoint(""));
        eq("icone sun", 0xE472, IconRules.codepoint("sun")); eq("icone cloud-sun", 0xE540, IconRules.codepoint("cloud-sun"));
        eq("icone cloud-moon", 0xE53E, IconRules.codepoint("cloud-moon")); eq("icone cloud", 0xE1AA, IconRules.codepoint("cloud"));
        eq("icone cloud-fog", 0xE53C, IconRules.codepoint("cloud-fog")); eq("icone cloud-rain", 0xE1B4, IconRules.codepoint("cloud-rain"));
        eq("icone cloud-lightning", 0xE1B2, IconRules.codepoint("cloud-lightning")); eq("icone cloud-snow", 0xE1B8, IconRules.codepoint("cloud-snow"));
        eq("icone drop", 0xE210, IconRules.codepoint("drop")); eq("icone plug", 0xE946, IconRules.codepoint("plug")); eq("icone plugs", 0xEB56, IconRules.codepoint("plugs"));
        eq("icone speaker-hifi", 0xEA08, IconRules.codepoint("speaker-hifi")); eq("icone plus", 0xE3D4, IconRules.codepoint("plus"));
        eq("icone caret-down", 0xE136, IconRules.codepoint("caret-down")); eq("icone check", 0xE182, IconRules.codepoint("check"));
        eq("36 icones", 36, IconRules.NAMES.length);
        eq("icone x-circle", 0xE4F8, IconRules.codepoint("x-circle")); eq("icone stack", 0xE466, IconRules.codepoint("stack")); eq("icone broom", 0xEC54, IconRules.codepoint("broom"));
        eq("icone monitor", 0xE32E, IconRules.codepoint("monitor")); eq("icone speaker-high", 0xE44A, IconRules.codepoint("speaker-high"));
        eq("icone sliders-horizontal", 0xE434, IconRules.codepoint("sliders-horizontal"));
        int[] nc = {0xE0A0, 0xE83C, 0xE4A6, 0xE6EE, 0xE6C8, 0xE038, 0xE13A, 0xE5DE};
        String[] nn = {"arrows-left-right", "swap", "trash", "text-aa", "palette", "arrow-counter-clockwise", "caret-right", "arrow-square-out"};
        for (int i = 0; i < nn.length; i++) eq("icone " + nn[i], nc[i], IconRules.codepoint(nn[i]));
        eq("meteo ciel clair jour", "sun", IconRules.weatherIcon(0, false)); eq("meteo ciel clair nuit", "moon", IconRules.weatherIcon(0, true));
        eq("meteo peu nuageux jour", "cloud-sun", IconRules.weatherIcon(1, false)); eq("meteo peu nuageux nuit", "cloud-moon", IconRules.weatherIcon(2, true));
        eq("meteo couvert", "cloud", IconRules.weatherIcon(3, false)); eq("meteo couvert nuit", "cloud", IconRules.weatherIcon(3, true));
        eq("meteo brouillard", "cloud-fog", IconRules.weatherIcon(45, false)); eq("meteo brouillard givrant", "cloud-fog", IconRules.weatherIcon(48, true));
        eq("meteo bruine", "cloud-rain", IconRules.weatherIcon(53, false)); eq("meteo pluie", "cloud-rain", IconRules.weatherIcon(63, true));
        eq("meteo averses", "cloud-rain", IconRules.weatherIcon(81, false)); eq("meteo neige", "cloud-snow", IconRules.weatherIcon(73, false));
        eq("meteo averses de neige", "cloud-snow", IconRules.weatherIcon(86, true)); eq("meteo orage", "cloud-lightning", IconRules.weatherIcon(95, false));
        eq("meteo orage grele", "cloud-lightning", IconRules.weatherIcon(99, true)); eq("meteo inconnu", "cloud", IconRules.weatherIcon(-1, false));
        eq("meteo code 4", "cloud", IconRules.weatherIcon(4, false));
        // nuit : lever / coucher du jour, sinon 7 h - 19 h
        long d0 = WeatherRules.local("2026-10-25T00:00"), hh = 3_600_000L;
        long[] up = {WeatherRules.local("2026-10-25T07:45"), WeatherRules.local("2026-10-26T07:46")}, dn = {WeatherRules.local("2026-10-25T18:10"), WeatherRules.local("2026-10-26T18:08")};
        ok("nuit: avant le lever", WeatherRules.isNight(d0 + 7 * hh, up, dn)); ok("jour: apres le lever", !WeatherRules.isNight(d0 + 8 * hh, up, dn));
        ok("jour: 18 h avant le coucher", !WeatherRules.isNight(d0 + 18 * hh, up, dn)); ok("nuit: 18 h 10 pile", WeatherRules.isNight(d0 + 18 * hh + 10 * 60_000L, up, dn));
        ok("nuit: minuit", WeatherRules.isNight(d0, up, dn)); ok("jour 2: 7 h 50 le 26", !WeatherRules.isNight(d0 + 24 * hh + 7 * hh + 50 * 60_000L, up, dn));
        ok("repli: 6 h nuit", WeatherRules.isNight(d0 + 6 * hh, null, null)); ok("repli: 7 h jour", !WeatherRules.isNight(d0 + 7 * hh, null, null));
        ok("repli: 18 h jour", !WeatherRules.isNight(d0 + 18 * hh + 59 * 60_000L, null, null)); ok("repli: 19 h nuit", WeatherRules.isNight(d0 + 19 * hh, null, null));
        ok("repli: jour absent du cache", !WeatherRules.isNight(d0 + 72 * hh + 12 * hh, up, dn)); ok("repli: tableaux de longueurs differentes", WeatherRules.isNight(d0 + 3 * hh, up, new long[0]));
        ok("repli: entree illisible", !WeatherRules.isNight(d0 + 12 * hh, new long[]{-1}, new long[]{-1}));
        eq("table: autant de noms que de codes", IconRules.NAMES.length, IconRules.CODES.length);
                eq("glyphe nom", "\uE472", IconRules.glyph("sun")); eq("glyphe nom inconnu", "", IconRules.glyph("house"));
        java.util.Set<Integer> seen = new java.util.HashSet<>();
        for (int c : IconRules.CODES) seen.add(c);
        eq("codepoints distincts", IconRules.CODES.length, seen.size());
    }

    // ---------- Libelles de nuit, valeur adb du theme
    static void nightLabels() {
        eq("jour clair fr", "Ensoleillé", Wmo.label(0, true, false)); eq("nuit claire fr", "Dégagé", Wmo.label(0, true, true));
        eq("jour clair en", "Clear", Wmo.label(0, false, false)); eq("nuit claire en", "Clear", Wmo.label(0, false, true));
        eq("peu nuageux nuit", "Peu nuageux", Wmo.label(1, true, true)); eq("peu nuageux nuit 2", "Partly cloudy", Wmo.label(2, false, true));
        eq("couvert nuit", "Couvert", Wmo.label(3, true, true)); eq("pluie nuit", "Pluie", Wmo.label(63, true, true));
        eq("orage jour", "Orage", Wmo.label(95, true, false));
        eq("ligne de veille de nuit", "12° ☾ Dégagé  ·  8° / 14°", StandbyRules.weatherLine(12, 0, 8, 14, true, "☾", true));
        eq("ligne de veille de jour", "12° ☀ Ensoleillé  ·  8° / 14°", StandbyRules.weatherLine(12, 0, 8, 14, true, "☀", false));
        eq("ligne de veille nuit en", "12° ☾ Clear", StandbyRules.weatherLine(12, 0, Double.NaN, Double.NaN, false, "☾", true));
        eq("adb night", "night", ThemeRules.fromSetting("night")); eq("adb sand", "sand", ThemeRules.fromSetting("sand"));
        eq("adb forest", "forest", ThemeRules.fromSetting("forest")); eq("adb auto", "auto", ThemeRules.fromSetting("auto"));
        eq("adb glass", "glass", ThemeRules.fromSetting("glass")); eq("adb casse + espaces + saut de ligne", "night", ThemeRules.fromSetting("  Night\n"));
        eq("adb majuscules", "forest", ThemeRules.fromSetting("FOREST"));
        ok("adb inconnu -> ignore", ThemeRules.fromSetting("rose") == null); ok("adb vide -> ignore", ThemeRules.fromSetting("") == null);
        ok("adb blanc -> ignore", ThemeRules.fromSetting("   ") == null); ok("adb null -> ignore", ThemeRules.fromSetting(null) == null);
        ok("adb avec guillemets -> ignore", ThemeRules.fromSetting("\"night\"") == null);
    }

    // ---------- Themes
    static void themeRules() {
        eq("theme: nom", "night", ThemeRules.normalize("night")); eq("theme: casse et espaces", "sand", ThemeRules.normalize("  SAND "));
        eq("theme: foret", "forest", ThemeRules.normalize("Forest")); eq("theme: auto", "auto", ThemeRules.normalize("AUTO"));
        eq("theme: inconnu -> verre", "glass", ThemeRules.normalize("rose")); eq("theme: null -> verre", "glass", ThemeRules.normalize(null));
        eq("theme: vide -> verre", "glass", ThemeRules.normalize(""));
        eq("libelle verre", "Verre", ThemeRules.label("glass", true)); eq("libelle glass", "Glass", ThemeRules.label("glass", false));
        eq("libelle nuit", "Nuit", ThemeRules.label("night", true)); eq("libelle sable", "Sable", ThemeRules.label("sand", true));
        eq("libelle foret", "Forêt", ThemeRules.label("forest", true)); eq("libelle forest", "Forest", ThemeRules.label("forest", false));
        eq("libelle auto", "Auto", ThemeRules.label("auto", true)); eq("libelle inconnu", "Verre", ThemeRules.label("zz", true));

        ThemeRules.Pal g = ThemeRules.glass();   // valeurs d'origine : le theme Verre ne change rien
        eq("verre dock", 0x38FFFFFF, g.glassFill); eq("verre liseré", 0x40FFFFFF, g.glassStroke); eq("verre panneau", 0x8C101216, g.panelFill);
        eq("verre carte", 0x26FFFFFF, g.cardFill); eq("verre pastille", 0x66000000, g.pillFill); eq("verre bulle", 0xE6101216, g.bubbleFill);
        eq("verre menu", 0xF21C1E24, g.overlayFill); eq("verre bande", 0xCC1C1E24, g.stripFill); eq("verre case +", 0x1FFFFFFF, g.tileFill);
        eq("verre tuile appli", 0x33FFFFFF, g.appFill);
        eq("palette glass", g.panelFill, ThemeRules.palette("glass", -1).panelFill); eq("palette inconnue = verre", g.panelFill, ThemeRules.palette("zz", 5).panelFill);

        ThemeRules.Pal n = ThemeRules.palette("night", -1);
        eq("nuit panneau = teinte", 0xB41E3470, n.panelFill); eq("nuit dock alpha", 0x80, n.glassFill >>> 24);
        eq("nuit menu", 0xF5182A5A, n.overlayFill); eq("nuit bulle", 0xF0182A5A, n.bubbleFill);
        eq("sable panneau", 0xB46A5030, ThemeRules.palette("sand", -1).panelFill); eq("foret panneau", 0xB41E5238, ThemeRules.palette("forest", -1).panelFill);
        eq("auto sans photo = neutre", ThemeRules.tinted(0x23262E).glassFill, ThemeRules.palette("auto", -1).glassFill);
        eq("auto avec teinte", 0xB4203040, ThemeRules.palette("auto", 0x203040).panelFill);
        ok("themes distincts", n.glassFill != ThemeRules.palette("sand", -1).glassFill && n.glassFill != ThemeRules.palette("forest", -1).glassFill);
        eq("alphas identiques entre themes teintes", ThemeRules.palette("sand", -1).overlayFill >>> 24, ThemeRules.palette("forest", -1).overlayFill >>> 24);
        // lisibilite : le texte blanc garde du contraste meme sur une photo blanche
        for (String nm : new String[]{"glass", "night", "sand", "forest", "auto"}) {
            ThemeRules.Pal q = ThemeRules.palette(nm, 0x203040);
            ok(nm + ": panneau lisible sur blanc", ThemeRules.contrast(0xFFFFFF, ThemeRules.over(q.panelFill, 0xFFFFFF)) >= 3.2);
            ok(nm + ": menu lisible sur blanc", ThemeRules.contrast(0xFFFFFF, ThemeRules.over(q.overlayFill, 0xFFFFFF)) >= 7);
            ok(nm + ": bulle lisible sur blanc", ThemeRules.contrast(0xFFFFFF, ThemeRules.over(q.bubbleFill, 0xFFFFFF)) >= 7);
            ok(nm + ": bande lisible sur blanc", ThemeRules.contrast(0xFFFFFF, ThemeRules.over(q.stripFill, 0xFFFFFF)) >= 5.5);
        }
        eq("contraste noir / blanc", 21.0, Math.round(ThemeRules.contrast(0x000000, 0xFFFFFF)) + 0.0);
        ok("luminance blanc", Math.abs(ThemeRules.luminance(0xFFFFFF) - 1.0) < 1e-6); ok("luminance noir", ThemeRules.luminance(0) == 0.0);

        eq("mix 0", 0x10203040, ThemeRules.mix(0x10203040, 0xF0E0D0C0, 0f)); eq("mix 1", 0xF0E0D0C0, ThemeRules.mix(0x10203040, 0xF0E0D0C0, 1f));
        eq("mix milieu", 0x80808080, ThemeRules.mix(0x00000000, 0xFFFFFFFF, 0.5f)); eq("mixRgb milieu", 0x808080, ThemeRules.mixRgb(0x000000, 0xFFFFFF, 0.5f));
        eq("mix palettes milieu", ThemeRules.mix(g.panelFill, n.panelFill, 0.5f), ThemeRules.mix(g, n, 0.5f).panelFill);
        eq("mix palettes 0", g.glassFill, ThemeRules.mix(g, n, 0f).glassFill); eq("mix palettes 1", n.glassFill, ThemeRules.mix(g, n, 1f).glassFill);

        eq("moyenne un pixel", 0x336699, ThemeRules.average(new int[]{0xFF336699}));
        eq("moyenne deux pixels identiques", 0x336699, ThemeRules.average(new int[]{0xFF336699, 0xFF336699}));
        int mixed = ThemeRules.average(new int[]{0xFF808080, 0xFFFF0000});   // le rouge sature pese plus que le gris
        ok("moyenne ponderee vers le colore", ((mixed >> 16) & 255) > 200 && (mixed & 255) < 40);
        eq("moyenne vide", -1, ThemeRules.average(new int[0])); eq("moyenne null", -1, ThemeRules.average(null));
        eq("moyenne noir", 0, ThemeRules.average(new int[]{0xFF000000}));
        int[] samples = {0xFFFFFF, 0x000000, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0x00FFFF, 0xFF00FF, 0x808080, 0xFFE0A0, 0x102030, 0xC0C0C0, 0x7F7F00, 0x010101};
        for (int c : samples) {
            int tn = ThemeRules.autoTint(c), r = (tn >> 16) & 255, gg = (tn >> 8) & 255, b = tn & 255, mx = Math.max(r, Math.max(gg, b)), mn = Math.min(r, Math.min(gg, b));
            String id = "auto " + Integer.toHexString(c);
            ok(id + ": sombre", ThemeRules.luminance(tn) <= 0.14); ok(id + ": luminosite 0,18-0,34", mx >= 44 && mx <= 89);
            ok(id + ": saturation 0,25-0,55", (mx - mn) / (double) mx >= 0.34 && (mx - mn) / (double) mx <= 0.66);
            ok(id + ": panneau lisible", ThemeRules.contrast(0xFFFFFF, ThemeRules.over(ThemeRules.tinted(tn).panelFill, 0xFFFFFF)) >= 3.2);
        }
        int red = ThemeRules.autoTint(0xC82828), blue = ThemeRules.autoTint(0x2828C8), green = ThemeRules.autoTint(0x28C828);
        ok("auto garde la nuance rouge", ((red >> 16) & 255) > ((red >> 8) & 255) && ((red >> 16) & 255) > (red & 255));
        ok("auto garde la nuance bleue", (blue & 255) > ((blue >> 16) & 255) && (blue & 255) > ((blue >> 8) & 255));
        ok("auto garde la nuance verte", ((green >> 8) & 255) > ((green >> 16) & 255) && ((green >> 8) & 255) > (green & 255));
        int grey = ThemeRules.autoTint(0x808080);
        ok("auto gris -> bleu-gris", (grey & 255) > ((grey >> 16) & 255));
        eq("auto stable", ThemeRules.autoTint(0x336699), ThemeRules.autoTint(0x336699));
    }

    // ---------- Attributions
    static void attribution() {
        eq("tmdb fr", "Données et images : TMDB. Ce produit utilise l'API TMDB mais n'est ni approuvé ni certifié par TMDB.", Attribution.tmdb(true));
        eq("tmdb en", "Data and images: TMDB. This product uses the TMDB API but is not endorsed or certified by TMDB.", Attribution.tmdb(false));
        eq("meteo fr", "Météo : Open-Meteo (CC BY 4.0)", Attribution.weather(true));
        eq("meteo en", "Weather: Open-Meteo (CC BY 4.0)", Attribution.weather(false));
    }

    // ---------- Menus lateraux
    static String kinds(java.util.List<MenuRules.Row> l) {
        StringBuilder b = new StringBuilder();
        for (MenuRules.Row r : l) b.append(r.kind);
        return b.toString();
    }

    static void menuRules() {
        java.util.List<MenuRules.Row> g = new ArrayList<>();   // menu d'une tuile : 3 actions, separateur, ajouter, 2 sous-menus, titre
        g.add(MenuRules.action("Déplacer", "arrows-left-right")); g.add(MenuRules.action("Remplacer", "swap")); g.add(MenuRules.action("Retirer", "trash"));
        g.add(MenuRules.separator()); g.add(MenuRules.action("Ajouter", "plus"));
        g.add(MenuRules.submenu("Nombre d'applis", "6", "squares-four")); g.add(MenuRules.submenu("Thème", "Nuit", "palette")); g.add(MenuRules.action("Titre", "text-aa"));
        eq("types de lignes", "00030110", kinds(g));
        ok("separateur non focalisable", !MenuRules.focusable(g.get(3))); ok("action focalisable", MenuRules.focusable(g.get(0)));
        eq("hauteur ligne", 52, MenuRules.rowHeight(g.get(0))); eq("hauteur separateur", 13, MenuRules.rowHeight(g.get(3)));
        eq("hauteur totale", 7 * 52 + 13, MenuRules.height(g));
        eq("haut de la 1re ligne", 0, MenuRules.top(g, 0)); eq("haut de la 3e ligne", 104, MenuRules.top(g, 2));
        eq("haut apres le separateur", 3 * 52 + 13, MenuRules.top(g, 4)); eq("haut hors bornes", MenuRules.height(g), MenuRules.top(g, 99));
        eq("1er focus = 1re ligne", 0, MenuRules.firstFocus(g));
        eq("valeur du sous-menu", "6", g.get(5).value); eq("icone", "palette", g.get(6).icon); eq("libelle", "Titre", g.get(7).label);
        eq("suivant saute le separateur", 4, MenuRules.step(g, 2, 1)); eq("precedent saute le separateur", 2, MenuRules.step(g, 4, -1));
        eq("suivant boucle", 0, MenuRules.step(g, 7, 1)); eq("precedent boucle", 7, MenuRules.step(g, 0, -1));
        eq("suivant sans focalisable", -1, MenuRules.step(java.util.Collections.singletonList(MenuRules.separator()), 0, 1));
        eq("une seule ligne boucle sur elle-meme", 0, MenuRules.step(java.util.Collections.singletonList(MenuRules.action("x", "")), 0, 1));
        ok("icones presentes", MenuRules.hasIcons(g)); ok("sans icones", !MenuRules.hasIcons(java.util.Collections.singletonList(MenuRules.action("x", ""))));
        ok("pastille compte comme icone", MenuRules.hasIcons(java.util.Collections.singletonList(MenuRules.choice("x", false, 0x123456))));
        eq("rien a focaliser", -1, MenuRules.firstFocus(java.util.Collections.singletonList(MenuRules.separator())));
        eq("menu de 10 lignes tient dans l'ecran", true, MenuRules.height(MenuRules.layoutRows(6, true)) + 2 * 24 + 110 < 720);

        java.util.List<MenuRules.Row> th = MenuRules.themeRows("night", 0x203040, true);   // themes
        eq("5 themes", 5, th.size()); eq("themes : choix", "22222", kinds(th));
        eq("libelle verre", "Verre", th.get(0).label); eq("libelle foret", "Forêt", th.get(3).label); eq("libelle auto", "Auto", th.get(4).label);
        ok("seul nuit est coche", !th.get(0).selected && th.get(1).selected && !th.get(2).selected && !th.get(3).selected && !th.get(4).selected);
        eq("le focus s'ouvre sur le theme actuel", 1, MenuRules.firstFocus(th));
        ok("chaque theme a une pastille", th.get(0).swatch >= 0 && th.get(1).swatch >= 0 && th.get(4).swatch >= 0);
        eq("pastille auto = teinte courante", ThemeRules.swatch("auto", 0x203040), th.get(4).swatch);
        eq("pastille nuit eclaircie", ThemeRules.mixRgb(0x1E3470, 0xFFFFFF, 0.25f), ThemeRules.swatch("night", -1));
        eq("pastille verre", 0xC9CED8, ThemeRules.swatch("glass", -1)); eq("pastille inconnue = verre", 0xC9CED8, ThemeRules.swatch("zz", -1));
        ok("theme inconnu -> verre coche", MenuRules.themeRows("zz", -1, false).get(0).selected);
        eq("libelle en", "Glass", MenuRules.themeRows("glass", -1, false).get(0).label);

        java.util.List<MenuRules.Row> ly = MenuRules.layoutRows(6, true);   // nombre d'applis
        eq("9 valeurs", 9, ly.size()); eq("premiere", "2 applis", ly.get(0).label); eq("derniere en", "10 apps", MenuRules.layoutRows(6, false).get(8).label);
        ok("6 est coche", ly.get(4).selected && !ly.get(3).selected); eq("focus sur 6", 4, MenuRules.firstFocus(ly));
        eq("focus sans valeur courante -> 1re ligne", 0, MenuRules.firstFocus(MenuRules.layoutRows(99, true)));

        java.util.List<MenuRules.Row> sl = MenuRules.sleepRows(60, true);   // minuteur
        eq("8 choix", 8, sl.size()); eq("desactive", "Désactivé", sl.get(0).label); eq("180", "180 min", sl.get(6).label); eq("240", "240 min", sl.get(7).label);
        ok("60 est coche", sl.get(3).selected); eq("focus sur 60", 3, MenuRules.firstFocus(sl));
        eq("minuteur inactif : Desactive coche", 0, MenuRules.firstFocus(MenuRules.sleepRows(0, true))); ok("Off en", MenuRules.sleepRows(0, false).get(0).label.equals("Off"));
        eq("aide fr", "◀  Retour        OK  Valider", MenuRules.help(true)); eq("aide en", "◀  Back        OK  Select", MenuRules.help(false));

        MenuRules.NavStack<String> st = new MenuRules.NavStack<>();   // pile de sous-menus
        eq("pile vide", 0, st.depth()); ok("pile vide : rien en haut", st.top() == null); ok("pile vide : pas de retour", !st.canPop());
        st.push("racine"); eq("racine", 1, st.depth()); ok("la racine ne se depile pas", !st.canPop()); eq("pop sur la racine", "racine", st.pop()); eq("racine toujours la", 1, st.depth());
        st.push("theme"); eq("sous-menu", "theme", st.top()); ok("retour possible", st.canPop()); eq("profondeur 2", 2, st.depth());
        st.push("sous-sous-menu"); eq("profondeur 3", 3, st.depth()); eq("pop -> sous-menu", "theme", st.pop()); eq("pop -> racine", "racine", st.pop());
        ok("de nouveau a la racine", !st.canPop()); eq("profondeur finale", 1, st.depth());

        // toutes les icones des menus sont dans la table
        for (String n : new String[]{"arrows-left-right", "swap", "trash", "text-aa", "palette", "arrow-counter-clockwise", "caret-right", "arrow-square-out"})
            ok("disponible : " + n, IconRules.codepoint(n) > 0 && !IconRules.glyph(n).isEmpty());
        for (String n : new String[]{"plus", "squares-four", "moon"}) ok("deja disponible : " + n, IconRules.codepoint(n) > 0);
    }

    // ---------- Applis ouvertes
    static java.util.Map<String, Long> usage(Object... kv) {
        java.util.Map<String, Long> m = new java.util.HashMap<>();
        for (int i = 0; i < kv.length; i += 2) m.put((String) kv[i], (Long) kv[i + 1]);
        return m;
    }

    static void recent() {
        for (String p : new String[]{"tv.zenhome", "android", "com.android.vending", "com.android.systemui", "com.android.tv.settings", "com.tcl.dashboard", "com.tcl.tvextras", "com.google.android.apps.tv.launcherx", "com.google.android.apps.tv.dreamx", "com.google.android.gms",
                "com.google.android.gms.persistent", "com.tcl.tv", "com.tcl.tvinput", "com.mediatek.wwtv.tvcenter", "", null})
            ok("protege : " + p, RecentRules.protectedPkg(p) && !RecentRules.canClose(p));
        for (String p : new String[]{"com.netflix.ninja", "com.google.android.youtube.tv", "tv.zenhome2", "com.google.android.apps.youtube", "org.xbmc.kodi", "com.google.android.gmsx"})
            ok("fermable : " + p, !RecentRules.protectedPkg(p) && RecentRules.canClose(p));

        long now = 10_000_000_000L, min = 60_000L, h3 = RecentRules.WINDOW_MS;
        java.util.Map<String, Long> u = usage("com.netflix.ninja", now - 5 * min, "org.xbmc.kodi", now - 20 * min, "tv.zenhome", now - min,
                "com.android.vending", now - 2 * min, "com.old.app", now - h3 - 1, "com.never", 0L, "com.future", now + 5 * 60_000L, "com.b", now - 20 * min);
        eq("tri par dernier usage, protegees et hors fenetre exclues, egalite par nom", "[com.netflix.ninja, com.b, org.xbmc.kodi]", RecentRules.select(u, now, h3, 10).toString());
        eq("plafond", "[com.netflix.ninja, com.b]", RecentRules.select(u, now, h3, 2).toString());
        eq("plafond 1", "[com.netflix.ninja]", RecentRules.select(u, now, h3, 1).toString());
        eq("pile 3 h incluse", "[com.edge]", RecentRules.select(usage("com.edge", now - h3), now, h3, 10).toString());
        eq("aucune appli", "[]", RecentRules.select(usage(), now, h3, 10).toString());
        java.util.Map<String, Long> many = new java.util.HashMap<>();
        for (int i = 0; i < 25; i++) many.put("com.app" + i, now - (i + 1) * min);
        eq("25 applis -> 10", 10, RecentRules.select(many, now, h3, RecentRules.MAX).size());
        eq("la plus recente en tete", "com.app0", RecentRules.select(many, now, h3, RecentRules.MAX).get(0));
        eq("la dixieme", "com.app9", RecentRules.select(many, now, h3, RecentRules.MAX).get(9));

        eq("a l'instant", "à l'instant", RecentRules.agoText(now, now - 30_000, true)); eq("just now", "just now", RecentRules.agoText(now, now - 59_999, false));
        eq("1 min", "il y a 1 min", RecentRules.agoText(now, now - min, true)); eq("5 min", "il y a 5 min", RecentRules.agoText(now, now - 5 * min, true));
        eq("59 min", "il y a 59 min", RecentRules.agoText(now, now - 59 * min - 59_000, true)); eq("5 min en", "5 min ago", RecentRules.agoText(now, now - 5 * min, false));
        eq("1 h", "il y a 1 h", RecentRules.agoText(now, now - 60 * min, true)); eq("2 h 59", "il y a 2 h", RecentRules.agoText(now, now - 179 * min, true));
        eq("2 h en", "2 h ago", RecentRules.agoText(now, now - 120 * min, false)); eq("2 jours", "il y a 2 j", RecentRules.agoText(now, now - 49 * 60 * min, true));
        eq("3 d en", "3 d ago", RecentRules.agoText(now, now - 72 * 60 * min, false)); eq("futur -> a l'instant", "à l'instant", RecentRules.agoText(now, now + 1000, true));

        eq("gain 312 Mo", 312L, RecentRules.freedMb(1_000_000, 1_000_000 + 312 * 1024)); eq("gain negatif -> 0", 0L, RecentRules.freedMb(2_000_000, 1_000_000));
        long b0 = 1_000_000, big = b0 + 312 * 1024, small = b0 + 19 * 1024;
        eq("3 applis + gain fr", "3 applis fermées  ·  312 Mo libérés", RecentRules.freeAllText(3, b0, big, true));
        eq("3 apps + gain en", "3 apps closed  ·  312 MB freed", RecentRules.freeAllText(3, b0, big, false));
        eq("1 appli fr", "1 appli fermée  ·  312 Mo libérés", RecentRules.freeAllText(1, b0, big, true)); eq("1 app en", "1 app closed", RecentRules.freeAllText(1, b0, b0, false));
        eq("gain faible : pas de Mo", "2 applis fermées", RecentRules.freeAllText(2, b0, small, true));
        eq("exactement 20 Mo", "2 applis fermées  ·  20 Mo libérés", RecentRules.freeAllText(2, b0, b0 + 20 * 1024, true));
        eq("gain negatif", "4 applis fermées", RecentRules.freeAllText(4, b0, b0 - 50_000, true));
        eq("mesure inconnue", "2 applis fermées", RecentRules.freeAllText(2, -1, -1, true));
        eq("rien a fermer fr", "Rien à fermer", RecentRules.freeAllText(0, b0, big, true)); eq("rien a fermer en", "Nothing to close", RecentRules.freeAllText(0, b0, big, false));
        eq("gain 312 Mo", 312L, RecentRules.freedMb(b0, big)); eq("gain negatif -> 0", 0L, RecentRules.freedMb(2_000_000, 1_000_000));
        eq("ram 0,8 Go", "RAM libre 0,8 Go", RecentRules.ramText(838_861, true)); eq("ram en", "Free RAM 0.8 GB", RecentRules.ramText(838_861, false));
        eq("ram 1,9 Go", "RAM libre 1,9 Go", RecentRules.ramText(2_000_000, true)); eq("ram 12 Go", "Free RAM 12 GB", RecentRules.ramText(12_582_912L, false));
        eq("ram inconnue", "", RecentRules.ramText(-1, true));
        eq("fermee fr", "Netflix fermée", RecentRules.closedText("Netflix", true, true)); eq("closed en", "Netflix closed", RecentRules.closedText("Netflix", true, false));
        eq("deja fermee fr", "Netflix est déjà fermée", RecentRules.closedText("Netflix", false, true)); eq("already closed en", "Netflix is already closed", RecentRules.closedText("Netflix", false, false));
        eq("inconnu fr", "Fermeture demandée : Netflix", RecentRules.closedText("Netflix", null, true)); eq("inconnu en", "Close requested: Netflix", RecentRules.closedText("Netflix", null, false));

        // lignes du menu : application, texte, focus apres suppression
        MenuRules.Row a = MenuRules.app("Netflix", "il y a 5 min");
        eq("ligne d'appli : sous-texte", "il y a 5 min", a.sub); eq("ligne d'appli : colonne d'icone", "app", a.icon); ok("ligne d'appli : focalisable", MenuRules.focusable(a));
        java.util.List<MenuRules.Row> rr = new ArrayList<>();
        rr.add(MenuRules.action("Tout libérer", "broom")); rr.add(MenuRules.app("A", "x")); rr.add(MenuRules.app("B", "y")); rr.add(MenuRules.app("C", "z"));
        eq("focus apres suppression : meme index", 2, MenuRules.nearestFocus(rr, 2)); eq("focus apres suppression en fin : borne", 3, MenuRules.nearestFocus(rr, 9));
        rr.remove(3); eq("derniere supprimee : la precedente", 2, MenuRules.nearestFocus(rr, 3));
        eq("focus negatif -> 0", 0, MenuRules.nearestFocus(rr, -4)); eq("liste vide", -1, MenuRules.nearestFocus(new ArrayList<MenuRules.Row>(), 0));
        java.util.List<MenuRules.Row> inf = new ArrayList<>();
        inf.add(MenuRules.action("Tout libérer", "broom")); inf.add(MenuRules.info("Autorisation requise : voir README"));
        ok("info non focalisable", !MenuRules.focusable(inf.get(1))); eq("info : le focus reste sur Tout liberer", 0, MenuRules.firstFocus(inf));
        eq("info : hauteur de ligne", 52, MenuRules.rowHeight(inf.get(1))); eq("info : precedent = Tout liberer", 0, MenuRules.step(inf, 0, 1));
        eq("info : focus apres suppression", 0, MenuRules.nearestFocus(inf, 1));
        java.util.List<MenuRules.Row> full = new ArrayList<>();
        full.add(MenuRules.action("Tout libérer", "broom"));
        for (int i = 0; i < 10; i++) full.add(a);
        ok("liste de 10 applis + Tout liberer tient dans le panneau (en-tete + aide ~119)", MenuRules.height(full) + 119 <= 672);
        eq("hauteur ligne d'appli", 46, MenuRules.rowHeight(a)); eq("hauteur ligne simple", 52, MenuRules.rowHeight(MenuRules.action("x", "")));
    }
}
