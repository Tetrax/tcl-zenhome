package tv.zenhome;

import android.content.ComponentName;
import android.content.Context;
import android.graphics.drawable.GradientDrawable;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Raccourcis (rangee d'icones blanches du panneau en haut a droite, sous les jauges) et bulle du libelle du raccourci qui a le focus. */
class Shortcuts {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final ViewGroup root;   // repere pour positionner la bulle
    final Host host;
    LinearLayout row;
    TextView bubble, sleepBadge;
    String sleepText = "";   // temps restant du minuteur (vide = aucun) : reapplique si la rangee est reconstruite

    Shortcuts(Context ctx, Ui ui, Apps apps, ViewGroup root, Host host) {
        this.ctx = ctx; this.ui = ui; this.apps = apps; this.root = root; this.host = host;
    }

    void buildRow(ViewGroup parent) {
        row = new LinearLayout(ctx);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setClipChildren(false); row.setClipToPadding(false);
        row.setPadding(ui.dp(6), ui.dp(2), ui.dp(6), ui.dp(4));
        parent.addView(row, new LinearLayout.LayoutParams(-2, -2));   // pas MATCH_PARENT (defaut d'une colonne) : la largeur suivrait le widget du dessus
        build();
    }

    void buildBubble(ViewGroup parent) {
        // bulle du libelle du raccourci qui a le focus (une seule, juste sous le bouton)
        bubble = ui.text("", 11, ui.fMed, 0xFFFFFFFF);
        bubble.setShadowLayer(0, 0, 0, 0);
        GradientDrawable sbb = new GradientDrawable(); sbb.setCornerRadius(ui.dp(10)); ui.theme.bind(sbb, Theme.BUBBLE, 0);
        bubble.setBackground(sbb);
        bubble.setPadding(ui.dp(10), ui.dp(5), ui.dp(10), ui.dp(5));
        bubble.setVisibility(View.GONE);
        parent.addView(bubble, new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.START));
    }

    // Raccourcis : entrees, toutes les applis, reglages, crayon (edition), fond, veille, Play Store
    void build() {
        row.removeAllViews();
        final String[] input = apps.findInput();
        final ComponentName viewInputs = apps.viewInputsTarget();
        // appli TV connue, ou (projecteurs XGIMI...) le systeme sait ouvrir les entrees
        if (input != null || viewInputs != null)
            shortcut(Icons.LineIcon.INPUT, ui.t("Entrées TV", "TV inputs"), new Runnable() { public void run() { host.showInputs(); } });
        shortcut(Icons.LineIcon.GRID, ui.t("Toutes les applis", "All apps"), new Runnable() { public void run() { host.showAllApps(); } });
        shortcut(Icons.LineIcon.GEAR, ui.t("Réglages", "Settings"), new Runnable() { public void run() { host.showSettings(); } });
        shortcut(Icons.LineIcon.PENCIL, ui.t("Modifier l'accueil", "Edit home screen"), new Runnable() { public void run() { host.editHome(); } });
        shortcut(Icons.LineIcon.IMAGE, ui.t("Fond", "Wallpaper"), new Runnable() { public void run() { host.nextWall(); } });
        FrameLayout moon = shortcut(Icons.LineIcon.MOON, ui.t("Mise en veille", "Sleep timer"), new Runnable() { public void run() { host.showSleep(); } });
        sleepBadge = ui.text("", 8, ui.fMed, 0xFF111111);
        sleepBadge.setShadowLayer(0, 0, 0, 0);
        sleepBadge.setPadding(ui.dp(4), ui.dp(1), ui.dp(4), ui.dp(1));
        GradientDrawable sbg = new GradientDrawable(); sbg.setCornerRadius(ui.dp(8)); sbg.setColor(Ui.ACCENT);
        sleepBadge.setBackground(sbg);
        moon.addView(sleepBadge, new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL));
        sleepBadge(sleepText);
        if (apps.usable("com.android.vending"))
            shortcut(Icons.LineIcon.PLAY, "Play Store", new Runnable() { public void run() { apps.launch("com.android.vending"); } });
    }

    /** Badge du temps restant sur la lune ; masque si vide. */
    void sleepBadge(String s) {
        sleepText = s;
        if (sleepBadge == null) return;
        if (!s.equals(String.valueOf(sleepBadge.getText()))) sleepBadge.setText(s);
        sleepBadge.setVisibility(s.isEmpty() ? View.GONE : View.VISIBLE);
    }

    /** Bouton de 44 unites : icone Phosphor Regular seule au repos ; au focus pastille blanche pleine et icone Fill sombre (zoom par Ui.wire, libelle par la bulle). */
    FrameLayout shortcut(int kind, String label, Runnable r) {
        FrameLayout f = new FrameLayout(ctx);
        final GradientDrawable bg = new GradientDrawable(); bg.setShape(GradientDrawable.OVAL); bg.setColor(0x00FFFFFF);
        f.setBackground(bg);
        final Icons.GlyphIcon glyph = Icons.GlyphIcon.make(ui.pReg, ui.pFill, IconRules.glyph(kind));   // Phosphor
        final Icons.LineIcon line = glyph == null ? new Icons.LineIcon(kind) : null;                 // repli si polices absentes
        ImageView iv = new ImageView(ctx); iv.setImageDrawable(glyph != null ? glyph : line);
        f.addView(iv, new FrameLayout.LayoutParams(ui.dp(24), ui.dp(24), Gravity.CENTER));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ui.dp(44), ui.dp(44)); lp.setMargins(ui.dp(1), 0, ui.dp(1), 0);
        f.setLayoutParams(lp);
        f.setTag(22);
        f.setContentDescription(label);
        ui.wire(f, r, 1.12f);
        final View.OnFocusChangeListener base = f.getOnFocusChangeListener();
        f.setOnFocusChangeListener(new View.OnFocusChangeListener() { public void onFocusChange(View v, boolean has) {
            base.onFocusChange(v, has);
            bg.setColor(has ? 0xFFFFFFFF : 0x00FFFFFF);
            if (glyph != null) glyph.focus(has); else line.setColor(has ? 0xFF111111 : 0xFFFFFFFF); } });
        f.setLongClickable(true);
        f.setOnLongClickListener(new View.OnLongClickListener() { public boolean onLongClick(View v) { host.showTileMenu(-1); return true; } });
        row.addView(f);
        return f;
    }

    /** Bulle du libelle sous le panneau, centree sur le bouton qui prend le focus sans sortir du panneau ; masquee quand il le perd. */
    void showBubble(View v, boolean has) {
        if (bubble == null) return;
        if (!has) { bubble.setVisibility(View.GONE); return; }
        bubble.setText(String.valueOf(v.getContentDescription()));
        bubble.measure(0, 0);
        View panel = (View) row.getParent();
        int[] a = new int[2], pa = new int[2], b = new int[2];
        v.getLocationInWindow(a); panel.getLocationInWindow(pa); root.getLocationInWindow(b);
        int w = bubble.getMeasuredWidth();
        int lo = pa[0] - b[0], hi = Math.max(lo, lo + panel.getWidth() - w);
        FrameLayout.LayoutParams lp = (FrameLayout.LayoutParams) bubble.getLayoutParams();
        lp.leftMargin = Math.max(lo, Math.min(a[0] - b[0] + v.getWidth() / 2 - w / 2, hi));
        lp.topMargin = pa[1] - b[1] + panel.getHeight() + ui.dp(6);
        bubble.setLayoutParams(lp);
        bubble.setVisibility(View.VISIBLE);
    }
}
