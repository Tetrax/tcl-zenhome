package tv.zenhome;

/**
 * Themes de couleur : palettes (verre, nuit, sable, foret, auto), teinte « auto » calculee depuis la photo, melange pour les
 * transitions (sans android.*). Une teinte est toujours sombre : les textes blancs restent lisibles.
 */
final class ThemeRules {
    private ThemeRules() { }

    static final String GLASS = "glass", NIGHT = "night", SAND = "sand", FOREST = "forest", AUTO = "auto";
    static final String[] NAMES = {GLASS, NIGHT, SAND, FOREST, AUTO};
    static final int NIGHT_TINT = 0x1E3470, SAND_TINT = 0x6A5030, FOREST_TINT = 0x1E5238;   // sombres mais nettement colorees
    static final int NEUTRAL_TINT = 0x23262E;   // teinte d'« auto » tant qu'aucune photo n'a ete echantillonnee

    /** Jetons de couleur d'un theme (ARGB). */
    static final class Pal {
        final int glassFill, glassStroke, panelFill, cardFill, pillFill, bubbleFill, overlayFill, stripFill, tileFill, appFill;
        Pal(int glassFill, int glassStroke, int panelFill, int cardFill, int pillFill, int bubbleFill, int overlayFill, int stripFill, int tileFill, int appFill) {
            this.glassFill = glassFill; this.glassStroke = glassStroke; this.panelFill = panelFill; this.cardFill = cardFill; this.pillFill = pillFill;
            this.bubbleFill = bubbleFill; this.overlayFill = overlayFill; this.stripFill = stripFill; this.tileFill = tileFill; this.appFill = appFill;
        }
    }

    /** Nom connu (sans casse ni espaces), sinon « glass ». */
    static String normalize(String s) {
        String n = s == null ? "" : s.trim().toLowerCase(java.util.Locale.ROOT);
        for (String k : NAMES) if (k.equals(n)) return k;
        return GLASS;
    }

    static String label(String name, boolean fr) {
        switch (normalize(name)) {
            case NIGHT: return fr ? "Nuit" : "Night";
            case SAND: return fr ? "Sable" : "Sand";
            case FOREST: return fr ? "Forêt" : "Forest";
            case AUTO: return "Auto";
            default: return fr ? "Verre" : "Glass";
        }
    }

    /** Le theme d'origine : verre neutre (les valeurs du code avant les themes). */
    static Pal glass() {
        return new Pal(0x38FFFFFF, 0x40FFFFFF, 0x8C101216, 0x26FFFFFF, 0x66000000, 0xE6101216, 0xF21C1E24, 0xCC1C1E24, 0x1FFFFFFF, 0x33FFFFFF);
    }

    /** Palette d'une teinte sombre (rgb) : memes alphas pour tous les themes teintes ; le panneau du haut prend la teinte telle quelle. */
    static Pal tinted(int rgb) {
        int light = mixRgb(rgb, 0xFFFFFF, 0.4f);
        return new Pal(argb(0x80, mixRgb(rgb, 0xFFFFFF, 0.12f)), 0x40FFFFFF, argb(0xB4, rgb), argb(0x59, light), argb(0xA0, shade(rgb, 0.7f)),
                argb(0xF0, shade(rgb, 0.8f)), argb(0xF5, shade(rgb, 0.8f)), argb(0xE0, shade(rgb, 0.8f)), argb(0x40, light), argb(0x59, light));
    }

    /** Palette du theme ; autoRgb : teinte « auto » courante (-1 : neutre). */
    static Pal palette(String name, int autoRgb) {
        switch (normalize(name)) {
            case NIGHT: return tinted(NIGHT_TINT);
            case SAND: return tinted(SAND_TINT);
            case FOREST: return tinted(FOREST_TINT);
            case AUTO: return tinted(autoRgb < 0 ? NEUTRAL_TINT : autoRgb);
            default: return glass();
        }
    }

    // ---------- couleurs

    static int argb(int a, int rgb) { return (a << 24) | (rgb & 0xFFFFFF); }

    static int shade(int rgb, float f) {
        return (Math.round(((rgb >> 16) & 255) * f) << 16) | (Math.round(((rgb >> 8) & 255) * f) << 8) | Math.round((rgb & 255) * f);
    }

    /** Melange canal par canal (alpha compris) : t = 0 donne a, t = 1 donne b. */
    static int mix(int a, int b, float t) {
        int r = 0;
        for (int s = 24; s >= 0; s -= 8) {
            int ca = (a >>> s) & 255, cb = (b >>> s) & 255;
            r |= Math.max(0, Math.min(255, Math.round(ca + (cb - ca) * t))) << s;
        }
        return r;
    }

    static int mixRgb(int a, int b, float t) { return mix(0xFF000000 | a, 0xFF000000 | b, t) & 0xFFFFFF; }

    static Pal mix(Pal a, Pal b, float t) {
        return new Pal(mix(a.glassFill, b.glassFill, t), mix(a.glassStroke, b.glassStroke, t), mix(a.panelFill, b.panelFill, t),
                mix(a.cardFill, b.cardFill, t), mix(a.pillFill, b.pillFill, t), mix(a.bubbleFill, b.bubbleFill, t),
                mix(a.overlayFill, b.overlayFill, t), mix(a.stripFill, b.stripFill, t), mix(a.tileFill, b.tileFill, t), mix(a.appFill, b.appFill, t));
    }

    /** Luminance relative (0..1) d'une couleur rgb. */
    static double luminance(int rgb) {
        double[] c = {(rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255};
        for (int i = 0; i < 3; i++) { double v = c[i] / 255.0; c[i] = v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); }
        return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    }

    /** Couleur argb posee sur un fond opaque rgb : le rgb resultant. */
    static int over(int argb, int bgRgb) { return mixRgb(bgRgb, argb & 0xFFFFFF, ((argb >>> 24) & 255) / 255f); }

    /** Rapport de contraste (1..21) entre deux couleurs rgb. */
    static double contrast(int a, int b) {
        double la = luminance(a), lb = luminance(b);
        return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
    }

    // ---------- teinte « auto »

    /**
     * Valeur de « settings put global zenhome_theme » (casse et espaces ignores) : le nom du theme, ou null si elle est vide ou inconnue
     * (le choix precedent est alors conserve).
     */
    static String fromSetting(String raw) {
        String n = raw == null ? "" : raw.trim().toLowerCase(java.util.Locale.ROOT);
        for (String k : NAMES) if (k.equals(n)) return k;
        return null;
    }

    /**
     * Moyenne ponderee (rgb) de pixels ARGB opaques : les pixels colores pesent plus que les gris (poids 0,25 + saturation) ;
     * -1 s'il n'y a aucun pixel.
     */
    static int average(int[] px) {
        if (px == null || px.length == 0) return -1;
        double sr = 0, sg = 0, sb = 0, sw = 0;
        for (int p : px) {
            int r = (p >> 16) & 255, g = (p >> 8) & 255, b = p & 255, mx = Math.max(r, Math.max(g, b)), mn = Math.min(r, Math.min(g, b));
            double w = 0.25 + (mx == 0 ? 0 : (mx - mn) / (double) mx);
            sr += r * w; sg += g * w; sb += b * w; sw += w;
        }
        return (Math.round((float) (sr / sw)) << 16) | (Math.round((float) (sg / sw)) << 8) | Math.round((float) (sb / sw));
    }

    /**
     * Teinte sombre dans la nuance de la couleur moyenne : saturation 0,35 a 0,65, luminosite 0,18 a 0,34 (toujours sombre).
     * Une moyenne grise prend une nuance bleu-gris.
     */
    static int autoTint(int rgb) {
        int r = (rgb >> 16) & 255, g = (rgb >> 8) & 255, b = rgb & 255;
        int mxI = Math.max(r, Math.max(g, b));
        float mx = mxI / 255f, mn = Math.min(r, Math.min(g, b)) / 255f, d = mx - mn;
        float s = mx == 0 ? 0 : d / mx, h;
        if (d < 0.02f || s < 0.06f) h = 220f;
        else if (mxI == r) h = 60f * (((g - b) / 255f / d) % 6);
        else if (mxI == g) h = 60f * ((b - r) / 255f / d + 2);
        else h = 60f * ((r - g) / 255f / d + 4);
        if (h < 0) h += 360f;
        s = Math.max(0.35f, Math.min(0.65f, s));
        float v = Math.max(0.18f, Math.min(0.34f, mx * 0.5f));
        float c = v * s, x = c * (1 - Math.abs((h / 60f) % 2 - 1)), m = v - c, rr, gg, bb;
        int sector = (int) (h / 60f) % 6;
        switch (sector) {
            case 0: rr = c; gg = x; bb = 0; break;
            case 1: rr = x; gg = c; bb = 0; break;
            case 2: rr = 0; gg = c; bb = x; break;
            case 3: rr = 0; gg = x; bb = c; break;
            case 4: rr = x; gg = 0; bb = c; break;
            default: rr = c; gg = 0; bb = x;
        }
        return (Math.round((rr + m) * 255) << 16) | (Math.round((gg + m) * 255) << 8) | Math.round((bb + m) * 255);
    }
}
