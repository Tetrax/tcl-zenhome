package tv.zenhome;

import android.animation.ValueAnimator;
import android.graphics.Bitmap;
import android.graphics.drawable.GradientDrawable;

import java.util.ArrayList;
import java.util.List;

/**
 * Theme de couleur courant (voir ThemeRules) : seul endroit qui connait les couleurs de verre. Les elements durables (dock, panneau du haut,
 * bulle, bandeau) sont lies avec bind() et suivent les changements de theme ; les menus et le panneau des entrees, recrees a chaque
 * ouverture, lisent la palette courante (p). Le theme « auto » prend la teinte de la photo affichee (echantillon de 24 x 14 pixels
 * calcule sur la bitmap deja chargee, aucune copie conservee), avec une transition de 600 ms.
 */
class Theme {
    static final int GLASS = 0, PANEL = 1, PILL = 2, BUBBLE = 3, OVERLAY = 4, STRIP = 5, CARD = 6, TILE = 7, APP = 8;
    static final long FADE_MS = 600;

    ThemeRules.Pal p = ThemeRules.glass();   // palette affichee (en transition : intermediaire)
    String name = ThemeRules.GLASS;
    int photoTint = -1;                      // derniere teinte calculee depuis la photo (-1 : aucune)
    ValueAnimator anim;

    static final class Bound {
        final GradientDrawable g; final int token, stroke;
        Bound(GradientDrawable g, int token, int stroke) { this.g = g; this.token = token; this.stroke = stroke; }
    }
    final List<Bound> bound = new ArrayList<>();

    int color(int token) {
        switch (token) {
            case PANEL: return p.panelFill;
            case PILL: return p.pillFill;
            case BUBBLE: return p.bubbleFill;
            case OVERLAY: return p.overlayFill;
            case STRIP: return p.stripFill;
            case CARD: return p.cardFill;
            case TILE: return p.tileFill;
            case APP: return p.appFill;
            default: return p.glassFill;
        }
    }

    /** Applique la couleur du jeton (et le liseré de verre si stroke > 0) a g, une fois. */
    void fill(GradientDrawable g, int token, int stroke) {
        g.setColor(color(token));
        if (stroke > 0) g.setStroke(stroke, p.glassStroke);
    }

    /** Comme fill(), et g suit ensuite les changements de theme (elements qui vivent autant que l'activite). */
    void bind(GradientDrawable g, int token, int stroke) {
        bound.add(new Bound(g, token, stroke));
        fill(g, token, stroke);
    }

    void applyAll() { for (Bound b : bound) fill(b.g, b.token, b.stroke); }

    /** Choix de l'utilisateur (nom inconnu : verre) ; changement immediat, sauf « auto » qui glisse vers la teinte de la photo. */
    void set(String n) {
        name = ThemeRules.normalize(n);
        goTo(ThemeRules.palette(name, photoTint), ThemeRules.AUTO.equals(name));
    }

    /** Une nouvelle photo (ou un backdrop) est affichee : on en echantillonne la teinte. */
    void onPhoto(Bitmap b) {
        if (b == null || b.isRecycled()) return;
        Bitmap s = null;
        try {
            s = Bitmap.createScaledBitmap(b, 24, 14, true);
            int[] px = new int[24 * 14];
            s.getPixels(px, 0, 24, 0, 0, 24, 14);
            int avg = ThemeRules.average(px);
            if (avg >= 0) photoTint = ThemeRules.autoTint(avg);
        } catch (Throwable ignored) {
        } finally { if (s != null && s != b) s.recycle(); }
        if (ThemeRules.AUTO.equals(name) && photoTint >= 0) goTo(ThemeRules.palette(name, photoTint), true);
    }

    void goTo(final ThemeRules.Pal to, boolean animate) {
        if (anim != null) anim.cancel();
        final ThemeRules.Pal from = p;
        if (!animate) { p = to; applyAll(); return; }
        anim = ValueAnimator.ofFloat(0f, 1f);
        anim.setDuration(FADE_MS);
        anim.addUpdateListener(new ValueAnimator.AnimatorUpdateListener() {
            public void onAnimationUpdate(ValueAnimator a) { p = ThemeRules.mix(from, to, (Float) a.getAnimatedValue()); applyAll(); } });
        anim.start();
    }
}
