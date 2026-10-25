package tv.zenhome;

import android.content.Context;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.animation.DecelerateInterpolator;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.List;

/**
 * Menu lateral (panneau a droite, verre teinte par Theme) pour tous les menus de l'accueil : titre + sous-titre, lignes de 52 unites
 * (icone Phosphor, libelle, valeur courante, chevron ou point de choix), surbrillance unique qui glisse d'une ligne a l'autre,
 * sous-menus qui glissent dans le meme panneau (pile), Retour = menu precedent puis fermeture. Les lignes sont decrites par
 * MenuRules (pur) ; les actions et les sous-menus (Source, reconstruit a chaque affichage : les valeurs sont toujours a jour) ici.
 */
class MenuPanel {
    static final int W = 400, MARGIN = 24, PAD = 12;
    static final long SLIDE_MS = 200, HL_MS = 140;

    /** Fabrique d'une page ; rappelee a chaque affichage (ouverture, retour d'un sous-menu). */
    interface Source { Page build(); }

    static class Item {
        final MenuRules.Row row; final Runnable action; final Source sub;
        Item(MenuRules.Row row, Runnable action, Source sub) { this.row = row; this.action = action; this.sub = sub; }
    }

    static class Page {
        final String title, subtitle;
        final List<Item> items = new ArrayList<>();
        Page(String title, String subtitle) { this.title = title; this.subtitle = subtitle; }
        Page add(MenuRules.Row r, Runnable a) { items.add(new Item(r, a, null)); return this; }
        Page sub(MenuRules.Row r, Source s) { items.add(new Item(r, null, s)); return this; }
        Page sep() { items.add(new Item(MenuRules.separator(), null, null)); return this; }
        /** Une ligne par element de rows, actions dans le meme ordre (listes de choix). */
        Page addAll(List<MenuRules.Row> rows, List<Runnable> actions) {
            for (int i = 0; i < rows.size(); i++) add(rows.get(i), actions.get(i));
            return this;
        }
        List<MenuRules.Row> rows() {
            List<MenuRules.Row> l = new ArrayList<>();
            for (Item it : items) l.add(it.row);
            return l;
        }
    }

    final Context ctx;
    final Ui ui;
    final Overlays ovs;
    final MenuRules.NavStack<Source> stack = new MenuRules.NavStack<>();
    FrameLayout ov, panel, pages;
    GradientDrawable panelBg;
    View cur;
    boolean busy;

    MenuPanel(Context ctx, Ui ui, Overlays ovs) { this.ctx = ctx; this.ui = ui; this.ovs = ovs; }

    void show(Source root) {
        ov = new FrameLayout(ctx);
        ov.setBackgroundColor(0x66000000);   // l'accueil reste visible, assombri
        ov.setClickable(true);
        panel = ui.roundedFrame(28, 0);
        panelBg = (GradientDrawable) panel.getBackground();
        style();
        LinearLayout body = new LinearLayout(ctx);
        body.setOrientation(LinearLayout.VERTICAL);
        pages = new FrameLayout(ctx);
        pages.setClipChildren(true);
        body.addView(pages, new LinearLayout.LayoutParams(-1, 0, 1f));
        TextView help = ui.text(MenuRules.help(ui.fr), 11, ui.fReg, 0x80FFFFFF);
        help.setShadowLayer(0, 0, 0, 0);
        help.setGravity(Gravity.CENTER);
        help.setPadding(0, ui.dp(8), 0, ui.dp(14));
        body.addView(help, new LinearLayout.LayoutParams(-1, -2));
        panel.addView(body, new FrameLayout.LayoutParams(-1, -1));
        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(ui.dp(W), -1, Gravity.END);
        lp.setMargins(0, ui.dp(MARGIN), ui.dp(MARGIN), ui.dp(MARGIN));
        ov.addView(panel, lp);

        stack.push(root);
        cur = buildPage(root.build());
        pages.addView(cur, new FrameLayout.LayoutParams(-1, -1));
        ovs.openOverlay(ov, (View) cur.getTag());
        ovs.onBack(ov, new Overlays.Back() { public boolean back() { return MenuPanel.this.back(); } });
        ovs.onExit(ov, new Runnable() { public void run() {
            panel.animate().translationX(ui.dp(W + MARGIN)).setDuration(160).start();
            ov.animate().alpha(0f).setDuration(160).start(); } });
        panel.setTranslationX(ui.dp(W + MARGIN));   // entree : glissement depuis la droite (le fondu est celui de l'overlay)
        panel.animate().translationX(0).setDuration(220).setInterpolator(new DecelerateInterpolator()).start();
    }

    /** Couleur du verre : palette courante du theme. */
    void style() { ui.theme.fill(panelBg, Theme.OVERLAY, Math.max(1, ui.dp(0.75f))); }

    // ---------- navigation

    void push(Source s) {
        if (busy) return;
        stack.push(s);
        slide(buildPage(s.build()), 1);
    }

    /** Retour : sous-menu precedent ; faux (le menu se ferme) sur la racine. */
    boolean back() {
        if (busy) return true;   // transition en cours : la touche est avalee
        if (!stack.canPop()) return false;
        Source s = stack.pop();
        style();
        slide(buildPage(s.build()), -1);
        return true;
    }

    /** next arrive depuis la droite (dir = 1) ou la gauche (dir = -1) ; la page courante part du cote oppose. */
    void slide(final View next, int dir) {
        final View old = cur;
        busy = true;
        cur = next;
        if (old instanceof android.view.ViewGroup) ((android.view.ViewGroup) old).setDescendantFocusability(android.view.ViewGroup.FOCUS_BLOCK_DESCENDANTS);
        pages.addView(next, new FrameLayout.LayoutParams(-1, -1));
        float w = ui.dp(W);
        next.setTranslationX(dir * w);
        next.animate().translationX(0).setDuration(SLIDE_MS).setInterpolator(new DecelerateInterpolator()).start();
        old.animate().translationX(-dir * w).setDuration(SLIDE_MS).setInterpolator(new DecelerateInterpolator())
                .withEndAction(new Runnable() { public void run() { pages.removeView(old); busy = false; } }).start();
        View f = (View) next.getTag();
        if (f != null) f.requestFocus();
    }

    // ---------- construction d'une page

    /** Vue d'une page ; son tag est la ligne qui prend le focus. */
    View buildPage(Page p) {
        final List<MenuRules.Row> rows = p.rows();
        LinearLayout pv = new LinearLayout(ctx);
        pv.setOrientation(LinearLayout.VERTICAL);
        pv.setPadding(ui.dp(PAD), ui.dp(22), ui.dp(PAD), 0);
        TextView title = ui.text(p.title, 20, ui.fMed, 0xFFFFFFFF);
        title.setShadowLayer(0, 0, 0, 0);
        title.setSingleLine(true); title.setEllipsize(android.text.TextUtils.TruncateAt.END);
        title.setPadding(ui.dp(14), 0, ui.dp(14), 0);
        pv.addView(title);
        if (!p.subtitle.isEmpty()) {
            TextView sub = ui.text(p.subtitle, 12, ui.fReg, 0x99FFFFFF);
            sub.setShadowLayer(0, 0, 0, 0);
            sub.setSingleLine(true); sub.setEllipsize(android.text.TextUtils.TruncateAt.END);
            sub.setPadding(ui.dp(14), ui.dp(3), ui.dp(14), 0);
            pv.addView(sub);
        }
        FrameLayout list = new FrameLayout(ctx);
        list.setClipChildren(false);
        LinearLayout.LayoutParams ll = new LinearLayout.LayoutParams(-1, -2); ll.topMargin = ui.dp(16);
        pv.addView(list, ll);
        final View hl = new View(ctx);   // la surbrillance unique : glisse d'une ligne a l'autre
        GradientDrawable hg = new GradientDrawable(); hg.setCornerRadius(ui.dp(16)); hg.setColor(0xFFFFFFFF);
        hl.setBackground(hg);
        hl.setVisibility(View.INVISIBLE);
        list.addView(hl, new FrameLayout.LayoutParams(-1, ui.dp(MenuRules.ROW_H)));
        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        list.addView(col, new FrameLayout.LayoutParams(-1, -2));

        boolean icons = MenuRules.hasIcons(rows);
        final View[] views = new View[rows.size()];
        for (int i = 0; i < rows.size(); i++) {
            Item it = p.items.get(i);
            MenuRules.Row r = it.row;
            if (r.kind == MenuRules.SEPARATOR) {
                FrameLayout s = new FrameLayout(ctx);
                View line = new View(ctx);
                line.setBackgroundColor(0x26FFFFFF);
                FrameLayout.LayoutParams ls = new FrameLayout.LayoutParams(-1, 1, Gravity.CENTER_VERTICAL);
                ls.setMargins(ui.dp(14), 0, ui.dp(14), 0);
                s.addView(line, ls);
                col.addView(s, new LinearLayout.LayoutParams(-1, ui.dp(MenuRules.SEP_H)));
                continue;
            }
            views[i] = row(it, i, rows, hl, icons);
            col.addView(views[i], new LinearLayout.LayoutParams(-1, ui.dp(MenuRules.ROW_H)));
            views[i].setId(View.generateViewId());
        }
        for (int i = 0; i < rows.size(); i++) {   // Haut / Bas bouclent entre les lignes focalisables ; Gauche / Droite restent dans le panneau
            if (views[i] == null) continue;
            views[i].setNextFocusUpId(views[MenuRules.step(rows, i, -1)].getId());
            views[i].setNextFocusDownId(views[MenuRules.step(rows, i, 1)].getId());
            views[i].setNextFocusLeftId(views[i].getId()); views[i].setNextFocusRightId(views[i].getId());
        }
        int f = MenuRules.firstFocus(rows);
        pv.setTag(f < 0 ? null : views[f]);
        return pv;
    }

    /** Une ligne focalisable : [icone ou pastille] libelle [valeur] [chevron ou point]. */
    View row(final Item it, final int index, final List<MenuRules.Row> rows, final View hl, boolean icons) {
        final MenuRules.Row r = it.row;
        LinearLayout v = new LinearLayout(ctx);
        v.setGravity(Gravity.CENTER_VERTICAL);
        v.setPadding(ui.dp(14), 0, ui.dp(14), 0);
        final TextView ic = ui.text(ui.pReg != null ? IconRules.glyph(r.icon) : "", 22, ui.pReg != null ? ui.pReg : ui.fReg, 0xF2FFFFFF);
        ic.setShadowLayer(0, 0, 0, 0);
        if (icons) {
            if (r.swatch >= 0) {   // pastille de la couleur du theme
                View sw = new View(ctx);
                GradientDrawable sg = new GradientDrawable(); sg.setShape(GradientDrawable.OVAL);
                sg.setColor(0xFF000000 | r.swatch); sg.setStroke(Math.max(1, ui.dp(1)), 0x99FFFFFF);
                sw.setBackground(sg);
                LinearLayout.LayoutParams ls = new LinearLayout.LayoutParams(ui.dp(18), ui.dp(18)); ls.rightMargin = ui.dp(14); ls.leftMargin = ui.dp(3);
                v.addView(sw, ls);
            } else {
                ic.setGravity(Gravity.CENTER);
                LinearLayout.LayoutParams li = new LinearLayout.LayoutParams(ui.dp(24), ui.dp(24)); li.rightMargin = ui.dp(14);
                v.addView(ic, li);
            }
        }
        final TextView label = ui.text(r.label, 15, ui.fMed, 0xFFFFFFFF);
        label.setShadowLayer(0, 0, 0, 0);
        label.setSingleLine(true); label.setEllipsize(android.text.TextUtils.TruncateAt.END);
        v.addView(label, new LinearLayout.LayoutParams(0, -2, 1f));
        final TextView val = ui.text(r.value, 13, ui.fReg, 0x99FFFFFF);
        val.setShadowLayer(0, 0, 0, 0);
        val.setSingleLine(true);
        LinearLayout.LayoutParams lv = new LinearLayout.LayoutParams(-2, -2); lv.leftMargin = ui.dp(10);
        if (!r.value.isEmpty()) v.addView(val, lv);
        final TextView caret = ui.text(ui.pReg != null && !IconRules.glyph("caret-right").isEmpty() ? IconRules.glyph("caret-right") : "›", 16,
                ui.pReg != null && !IconRules.glyph("caret-right").isEmpty() ? ui.pReg : ui.fReg, 0x99FFFFFF);
        caret.setShadowLayer(0, 0, 0, 0);
        if (r.kind == MenuRules.SUBMENU) {
            LinearLayout.LayoutParams lc = new LinearLayout.LayoutParams(-2, -2); lc.leftMargin = ui.dp(8);
            v.addView(caret, lc);
        }
        final GradientDrawable radio = new GradientDrawable(); radio.setShape(GradientDrawable.OVAL);
        if (r.kind == MenuRules.CHOICE) {
            View rv = new View(ctx);
            rv.setBackground(radio);
            LinearLayout.LayoutParams lr = new LinearLayout.LayoutParams(ui.dp(16), ui.dp(16)); lr.leftMargin = ui.dp(10);
            v.addView(rv, lr);
        }
        v.setFocusable(true); v.setClickable(true);
        final boolean sel = r.selected;
        v.setOnFocusChangeListener(new View.OnFocusChangeListener() { public void onFocusChange(View x, boolean has) {
            int dark = 0xFF111111;
            label.setTextColor(has ? dark : 0xFFFFFFFF);
            val.setTextColor(has ? 0xAA111111 : 0x99FFFFFF);
            caret.setTextColor(has ? 0xAA111111 : 0x99FFFFFF);
            ic.setTextColor(has ? dark : 0xF2FFFFFF);
            if (ui.pReg != null) ic.setTypeface(has ? ui.pFill : ui.pReg);   // Regular au repos, Fill au focus
            radio.setStroke(Math.max(1, ui.dp(1.5f)), has ? 0xAA111111 : 0xAAFFFFFF);
            radio.setColor(sel ? (has ? dark : 0xFFFFFFFF) : 0x00FFFFFF);
            if (has) moveHighlight(hl, topPx(rows, index));
        } });
        // etat de repos des couleurs du point de choix
        radio.setStroke(Math.max(1, ui.dp(1.5f)), 0xAAFFFFFF);
        radio.setColor(sel ? 0xFFFFFFFF : 0x00FFFFFF);
        v.setOnClickListener(new View.OnClickListener() { public void onClick(View x) { activate(it); } });
        v.setOnKeyListener(new View.OnKeyListener() { public boolean onKey(View x, int code, KeyEvent e) {
            if (code == KeyEvent.KEYCODE_DPAD_LEFT) {   // ◀ = Retour
                if (e.getAction() == KeyEvent.ACTION_DOWN && e.getRepeatCount() == 0) ovs.closeTopOverlay();
                return true;
            }
            return false; } });
        return v;
    }

    /** Haut de la ligne idx en pixels : somme des hauteurs arrondies comme les lignes elles-memes (pas d'ecart cumule). */
    int topPx(List<MenuRules.Row> rows, int idx) {
        int y = 0;
        for (int i = 0; i < idx && i < rows.size(); i++) y += ui.dp(MenuRules.rowHeight(rows.get(i)));
        return y;
    }

    /** La surbrillance se place sans animation la premiere fois, puis glisse (translationY, 140 ms). */
    void moveHighlight(View hl, int y) {
        if (hl.getVisibility() != View.VISIBLE) { hl.setTranslationY(y); hl.setVisibility(View.VISIBLE); return; }
        hl.animate().translationY(y).setDuration(HL_MS).setInterpolator(new DecelerateInterpolator()).start();
    }

    void activate(Item it) {
        if (busy) return;
        switch (it.row.kind) {
            case MenuRules.SUBMENU: push(it.sub); break;
            case MenuRules.CHOICE:
                if (stack.canPop()) { it.action.run(); back(); }   // le choix s'applique, le sous-menu se referme sur son parent mis a jour
                else ovs.closeThen(ov, it.action);
                break;
            default: ovs.closeThen(ov, it.action);
        }
    }
}
