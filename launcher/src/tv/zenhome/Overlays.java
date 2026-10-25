package tv.zenhome;

import android.content.Context;
import android.graphics.drawable.GradientDrawable;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

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

    void showMenu(String header, List<String> labels, final List<Runnable> actions) { showMenu(header, labels, actions, 0); }

    /** focusIdx : ligne qui prend le focus a l'ouverture. */
    void showMenu(String header, List<String> labels, final List<Runnable> actions, int focusIdx) {
        final FrameLayout ov = new FrameLayout(ctx);
        ov.setBackgroundColor(0x99000000);
        ov.setClickable(true);
        LinearLayout panel = new LinearLayout(ctx);
        panel.setOrientation(LinearLayout.VERTICAL);
        GradientDrawable pb = new GradientDrawable(); pb.setCornerRadius(ui.dp(22)); ui.theme.fill(pb, Theme.OVERLAY, Math.max(1, ui.dp(0.75f)));
        panel.setBackground(pb);
        panel.setPadding(ui.dp(14), ui.dp(18), ui.dp(14), ui.dp(14));
        panel.setClipChildren(false); panel.setClipToPadding(false);
        TextView hd = ui.text(header, 16, ui.fMed, 0xFFFFFFFF); hd.setShadowLayer(0, 0, 0, 0);
        hd.setPadding(ui.dp(12), 0, ui.dp(12), ui.dp(10)); hd.setSingleLine(true); hd.setEllipsize(TextUtils.TruncateAt.END);
        panel.addView(hd);
        View first = null, last = null, start = null;
        for (int i = 0; i < labels.size(); i++) {
            final Runnable a = actions.get(i);
            TextView row = ui.text(labels.get(i), 13, ui.fReg, 0xFFFFFFFF); row.setShadowLayer(0, 0, 0, 0);
            row.setText(ui.checkText(labels.get(i)));
            row.setPadding(ui.dp(14), ui.dp(10), ui.dp(14), ui.dp(10));
            row.setSingleLine(true);
            final GradientDrawable rb = new GradientDrawable(); rb.setCornerRadius(ui.dp(12)); rb.setColor(0x00FFFFFF);
            row.setBackground(rb);
            row.setFocusable(true); row.setClickable(true);
            row.setOnFocusChangeListener(new View.OnFocusChangeListener() { public void onFocusChange(View v, boolean has) {
                rb.setColor(has ? 0xFFFFFFFF : 0x00FFFFFF); ((TextView) v).setTextColor(has ? 0xFF111111 : 0xFFFFFFFF); } });
            row.setOnClickListener(new View.OnClickListener() { public void onClick(View v) { closeThen(ov, a); } });
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(-1, -2); lp.setMargins(0, ui.dp(2), 0, ui.dp(2));
            panel.addView(row, lp);
            row.setId(View.generateViewId());
            row.setNextFocusLeftId(row.getId()); row.setNextFocusRightId(row.getId());   // ◀ ▶ ne sortent pas du panneau
            if (last != null) { last.setNextFocusDownId(row.getId()); row.setNextFocusUpId(last.getId()); }
            if (first == null) first = row;
            if (i == focusIdx) start = row;
            last = row;
        }
        if (first != null) { last.setNextFocusDownId(first.getId()); first.setNextFocusUpId(last.getId()); }   // navigation bouclee
        FrameLayout.LayoutParams lpp = new FrameLayout.LayoutParams(ui.dp(360), -2, Gravity.CENTER);
        ov.addView(panel, lpp);
        openOverlay(ov, start != null ? start : first);
    }

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
        root.removeView(ov);
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
        closeOverlay(ov, true);
    }
}
