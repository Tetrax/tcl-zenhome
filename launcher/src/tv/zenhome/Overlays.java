package tv.zenhome;

import android.content.Context;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;

/**
 * Pile des menus / listes en surimpression. Tant qu'un overlay est ouvert, `content` (tout le fond)
 * ne peut plus recevoir le focus ; seul l'overlay du dessus le peut.
 */
class Overlays {
    // retour de focus apres un overlay : on memorise la vue (sa rangee / position, et le niveau de l'accueil) qui l'a ouvert
    static class Origin { View v; ViewGroup row; int idx; Nav.State level = Nav.State.REST; }

    final Context ctx;
    final Ui ui;
    final ViewGroup root, content;
    final Host host;
    final List<View> overlays = new ArrayList<>();   // menus / listes ouverts (pile)
    final HashMap<View, Origin> origins = new HashMap<>();
    Origin pendingOrigin;            // origine transmise d'un menu ferme a l'overlay qu'il ouvre

    Overlays(Context ctx, Ui ui, ViewGroup root, ViewGroup content, Host host) {
        this.ctx = ctx; this.ui = ui; this.root = root; this.content = content; this.host = host;
    }

    boolean isEmpty() { return overlays.isEmpty(); }

    /** Overlay du dessus (la pile ne doit pas etre vide). */
    View top() { return overlays.get(overlays.size() - 1); }

    /** Retour sur un overlay a pages (menu lateral) : vrai si la touche est consommee (page precedente), faux pour fermer l'overlay. */
    interface Back { boolean back(); }

    final HashMap<View, Back> backs = new HashMap<>();
    final HashMap<View, Runnable> exits = new HashMap<>();   // animation de sortie (l'overlay est retire 170 ms plus tard)

    void onBack(View ov, Back b) { backs.put(ov, b); }
    void onExit(View ov, Runnable r) { exits.put(ov, r); }

    /** Menu lateral (voir MenuPanel) ; root est rappele a chaque affichage de la page. */
    void showMenu(MenuPanel.Source root) { new MenuPanel(ctx, ui, this).show(root); }

    /** Ferme ov puis lance a : l'origine du focus est transmise a l'overlay que a ouvrirait. */
    void closeThen(View ov, Runnable a) {
        Origin o = origins.get(ov);
        closeOverlay(ov, false);
        pendingOrigin = o;
        try { a.run(); } finally { pendingOrigin = null; }
    }

    /** Seul le menu ou la liste du dessus peut recevoir le focus : les flèches ne doivent jamais sauter vers ce qui est derrière. */
    void updateFocusBlocking() {
        int n = overlays.size();
        // tout le fond (dock, bouton, pastilles, affiches, carrousel, raccourcis...) est un seul conteneur
        content.setDescendantFocusability(n == 0 ? ViewGroup.FOCUS_AFTER_DESCENDANTS : ViewGroup.FOCUS_BLOCK_DESCENDANTS);
        for (int i = 0; i < n; i++) {
            View o = overlays.get(i);
            if (o instanceof ViewGroup)
                ((ViewGroup) o).setDescendantFocusability(i == n - 1 ? ViewGroup.FOCUS_AFTER_DESCENDANTS : ViewGroup.FOCUS_BLOCK_DESCENDANTS);
        }
    }

    void openOverlay(View ov, View focus) {
        Origin from = !overlays.isEmpty() ? origins.get(overlays.get(0))
                : (pendingOrigin != null ? pendingOrigin : currentOrigin());
        origins.put(ov, from);
        root.addView(ov, new FrameLayout.LayoutParams(-1, -1));
        overlays.add(ov);
        updateFocusBlocking();
        ov.setAlpha(0f); ov.animate().alpha(1f).setDuration(160).start();
        if (focus != null) focus.requestFocus();
    }

    Origin currentOrigin() {
        View f = host.currentFocus();
        if (f == null) return null;
        Origin o = new Origin();
        o.v = f;
        o.row = f.getParent() instanceof ViewGroup ? (ViewGroup) f.getParent() : null;
        o.idx = o.row == null ? -1 : o.row.indexOfChild(f);
        o.level = host.level();
        return o;
    }

    /** Rend le focus a la vue qui avait ouvert l'overlay, sinon a la meme position de sa rangee, sinon a son niveau (repos si indisponible). */
    void restoreFocus(Origin o) {
        if (o != null) {
            if (o.v.isAttachedToWindow() && o.v.getVisibility() == View.VISIBLE && o.v.requestFocus()) return;
            if (o.row != null && o.row.isAttachedToWindow() && o.row.getChildCount() > 0
                    && o.row.getChildAt(Math.min(Math.max(o.idx, 0), o.row.getChildCount() - 1)).requestFocus()) return;
            host.focusLevel(o.level);
            return;
        }
        host.focusDock();
    }

    void closeOverlay(View ov, boolean refocus) {
        Origin from = origins.remove(ov);
        backs.remove(ov);
        Runnable exit = exits.remove(ov);
        if (exit == null) root.removeView(ov);
        else {   // sortie animee : plus aucun focus possible, retire a la fin du fondu
            if (ov instanceof ViewGroup) ((ViewGroup) ov).setDescendantFocusability(ViewGroup.FOCUS_BLOCK_DESCENDANTS);
            exit.run();
            final View gone = ov;
            root.postDelayed(new Runnable() { public void run() { root.removeView(gone); } }, 170);
        }
        overlays.remove(ov);
        host.onOverlayClosed(ov);
        updateFocusBlocking();
        if (refocus) {
            if (!overlays.isEmpty()) overlays.get(overlays.size() - 1).requestFocus();
            else restoreFocus(from);
        }
    }

    void closeTopOverlay() {
        if (overlays.isEmpty()) return;
        View ov = overlays.get(overlays.size() - 1);
        Back b = backs.get(ov);
        if (b != null && b.back()) return;   // sous-menu : retour a la page precedente
        closeOverlay(ov, true);
    }
}
