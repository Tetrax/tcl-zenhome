package tv.zenhome;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

/** Entrees TV (TvInputInfo) : filtre, libelle, etat et tri (sans android.*, les constantes sont celles de TvInputInfo / TvInputManager). */
final class InputRules {
    private InputRules() { }

    static final int TYPE_TUNER = 0, TYPE_HDMI = 1007;                            // TvInputInfo.TYPE_*
    static final int CONNECTED = 0, STANDBY = 1, DISCONNECTED = 2;                // TvInputManager.INPUT_STATE_*

    static final class Entry {
        final String id, label;
        final int type, state;
        Entry(String id, String label, int type, int state) { this.id = id; this.label = label; this.type = type; this.state = state; }
    }

    /** Entrees materielles seulement (HDMI, AV...) ; un tuner n'est garde que s'il vient du meme paquet qu'une entree materielle (pas les applis de chaines). */
    static boolean keep(boolean passthrough, int type, boolean vendorTuner) {
        return passthrough || (type == TYPE_TUNER && vendorTuner);
    }

    /** Carte « Entrees » du dock : une appli ou cible d'entrees connue ET au moins une entree physique visible (sinon Chaines en direct s'ouvrirait pour rien). */
    static boolean showCard(boolean target, boolean hasInputs) { return target && hasInputs; }

    static String clean(CharSequence s) { return s == null ? "" : s.toString().trim(); }

    /** Libelle personnalise (renomme dans les reglages TV) s'il existe, sinon libelle du systeme, sinon nom du type. */
    static String label(CharSequence custom, CharSequence loaded, int type, boolean fr) {
        String c = clean(custom), l = clean(loaded);
        if (!c.isEmpty()) return c;
        if (!l.isEmpty()) return l;
        if (type == TYPE_HDMI) return "HDMI";
        if (type == TYPE_TUNER) return fr ? "Antenne / TV" : "Tuner / TV";
        return fr ? "Entrée AV" : "AV input";
    }

    static Entry entry(String id, CharSequence custom, CharSequence loaded, int type, int state, boolean fr) {
        return new Entry(id, label(custom, loaded, type, fr), type, state);
    }

    static String stateText(int state, boolean fr) {
        if (state == CONNECTED) return fr ? "Branché" : "Connected";
        if (state == STANDBY) return fr ? "En veille" : "Standby";
        if (state == DISCONNECTED) return fr ? "Non branché" : "Not connected";
        return "";
    }

    /** Numero final de l'identifiant (« .../HW16 » -> 16), -1 s'il n'y en a pas. */
    static int number(String id) {
        if (id == null) return -1;
        int e = id.length(), b = e;
        while (b > 0 && Character.isDigit(id.charAt(b - 1))) b--;
        if (b == e || e - b > 6) return -1;
        return Integer.parseInt(id.substring(b));
    }

    private static int stateRank(int s) { return s == CONNECTED ? 0 : s == STANDBY ? 1 : 2; }

    private static int typeRank(int t) { return t == TYPE_HDMI ? 0 : t == TYPE_TUNER ? 2 : 1; }

    /** Branches d'abord, puis veille, puis non branches ; dans chaque groupe HDMI, autres entrees AV, tuner ; puis par numero. Tri stable. */
    static List<Entry> sort(List<Entry> in) {
        List<Entry> l = new ArrayList<>(in);
        Collections.sort(l, new Comparator<Entry>() {
            public int compare(Entry a, Entry b) {
                int d = stateRank(a.state) - stateRank(b.state);
                if (d == 0) d = typeRank(a.type) - typeRank(b.type);
                if (d == 0) d = number(a.id) - number(b.id);
                return d;
            }
        });
        return l;
    }

    // ---------- Cartes du panneau

    static final int GREEN = 0xFF4CD964, ORANGE = 0xFFFFB020, GREY = 0x80FFFFFF;

    static final String[] AUDIO_WORDS = {"soundbar", "sound bar", "barre de son", "cinema", "cinéma", "speaker", "enceinte", "hifi", "hi-fi", "receiver", "ampli"};

    /** Barre de son / ampli HDMI, d'apres le libelle (renomme par l'utilisateur ou donne par l'appareil). */
    static boolean looksAudio(String label) {
        String l = label == null ? "" : label.toLowerCase(java.util.Locale.ROOT);
        for (String w : AUDIO_WORDS) if (l.contains(w)) return true;
        for (String tok : l.split("[^a-z0-9]+")) if (tok.equals("avr") || tok.matches("sb\\d+")) return true;
        return false;
    }

    /** Nom d'icone Phosphor : prise (HDMI), prises (AV : composite, S-Video, peritel, composante), TV (tuner), enceinte (HDMI nomme comme une barre de son), sinon TV. */
    static String iconName(int type, String label) {
        if (type == TYPE_HDMI) return looksAudio(label) ? "speaker-hifi" : "plug";
        if (type >= 1001 && type <= 1004) return "plugs";
        return "television-simple";
    }

    /** Couleur de la pastille d'etat. */
    static int stateColor(int state) { return state == CONNECTED ? GREEN : state == STANDBY ? ORANGE : GREY; }

    /** Une entree non branchee est attenuee (mais reste selectionnable). */
    static float cardAlpha(int state) { return state == DISCONNECTED ? 0.5f : 1f; }

    /** Taille des cartes (unites 1280) pour `entries` entrees + la carte « Plus… » : meme geometrie que le dock. */
    static int cardCount(int entries) { return entries + 1; }
    static int cardW(int cards) { return DockMetrics.bigW(Math.max(2, cards)); }
    static int cardH(int cards) { return DockMetrics.bigH(Math.max(2, cards)); }
    static int cardM(int cards) { return DockMetrics.bigM(Math.max(2, cards)); }

    /** Carte qui prend le focus : la premiere entree branchee, sinon la premiere. */
    static int focusIndex(List<Entry> l) {
        for (int i = 0; i < l.size(); i++) if (l.get(i).state == CONNECTED) return i;
        return 0;
    }
}
