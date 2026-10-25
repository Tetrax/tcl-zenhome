package tv.zenhome;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.GridLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.List;

/** Dock : une seule rangee de `layout` tuiles (2 a 10), largeur calculee pour tenir dans l'ecran. */
class Dock {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final HomeConfig config;
    final Host host;
    LinearLayout view;
    List<View> bigTiles = new ArrayList<>();

    Dock(Context ctx, Ui ui, Apps apps, HomeConfig config, Host host) {
        this.ctx = ctx; this.ui = ui; this.apps = apps; this.config = config; this.host = host;
    }

    void build(ViewGroup parent) {
        view = new LinearLayout(ctx);
        view.setOrientation(LinearLayout.HORIZONTAL);
        view.setGravity(Gravity.CENTER_VERTICAL);
        view.setClipChildren(false); view.setClipToPadding(false);
        GradientDrawable glass = new GradientDrawable();
        glass.setCornerRadius(ui.dp(28));
        ui.theme.bind(glass, Theme.GLASS, Math.max(1, ui.dp(0.75f)));
        view.setBackground(glass);
        FrameLayout.LayoutParams lpD = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
        lpD.setMargins(0, 0, 0, ui.dp(46));
        parent.addView(view, lpD);
    }

    // Dock : une seule rangee de `layout` tuiles (2 a 10), largeur calculee pour tenir dans l'ecran
    int bigW() { return DockMetrics.bigW(config.layout); }
    int bigH() { return DockMetrics.bigH(config.layout); }
    int bigM() { return DockMetrics.bigM(config.layout); }
    boolean compactTiles() { return DockMetrics.compact(config.layout); }

    void rebuild(int focusIndex) {
        view.removeAllViews();
        bigTiles.clear();
        view.setPadding(ui.dp(DockMetrics.DOCK_PAD), ui.dp(18), ui.dp(DockMetrics.DOCK_PAD), ui.dp(18));

        GridLayout big = new GridLayout(ctx);
        big.setColumnCount(Math.max(1, config.dockPkgs.size() + (config.dockPkgs.size() < config.layout ? 1 : 0)));
        big.setClipChildren(false); big.setClipToPadding(false);
        for (int i = 0; i < config.dockPkgs.size() && i < config.layout; i++) {
            View v = bigTile(config.dockPkgs.get(i), i);
            if (v != null) { big.addView(v); bigTiles.add(v); }
        }
        if (config.dockPkgs.size() < config.layout) big.addView(addTile());   // case "+" pour ajouter une appli
        view.addView(big);

        host.onDockBuilt();

        final View target = (focusIndex >= 0 && focusIndex < bigTiles.size()) ? bigTiles.get(focusIndex)
                : (bigTiles.isEmpty() ? big.getChildAt(0) : bigTiles.get(bigTiles.size() - 1));
        if (target != null) view.post(new Runnable() { public void run() { target.requestFocus(); } });
    }


    View bigTile(final String pkg, final int index) {
        PackageManager pm = ctx.getPackageManager();
        Drawable d = null;
        String label = pkg;
        try {
            ApplicationInfo ai = pm.getApplicationInfo(pkg, 0);
            label = pm.getApplicationLabel(ai).toString();
            d = pm.getApplicationBanner(ai);
            if (d == null) { Intent li = pm.getLeanbackLaunchIntentForPackage(pkg); if (li != null) d = pm.getActivityBanner(li); }
        } catch (Exception e) { return null; }
        FrameLayout f = ui.roundedFrame(14, 0xFF1E1E1E);
        if (d != null && !compactTiles()) {
            ImageView iv = new ImageView(ctx);
            iv.setImageDrawable(d);
            iv.setScaleType(ImageView.ScaleType.CENTER_CROP);
            f.addView(iv, new FrameLayout.LayoutParams(-1, -1));
        } else {
            LinearLayout l = new LinearLayout(ctx);
            l.setOrientation(LinearLayout.VERTICAL); l.setGravity(Gravity.CENTER);
            ImageView iv = new ImageView(ctx); iv.setImageDrawable(apps.pkgIcon(pkg));
            int is = compactTiles() ? 30 : 44;
            LinearLayout.LayoutParams il = new LinearLayout.LayoutParams(ui.dp(is), ui.dp(is)); il.gravity = Gravity.CENTER_HORIZONTAL;
            l.addView(iv, il);
            TextView tv = ui.text(label, compactTiles() ? 9 : 11, ui.fMed, 0xFFFFFFFF); tv.setGravity(Gravity.CENTER); tv.setSingleLine(true);
            tv.setEllipsize(TextUtils.TruncateAt.END);
            tv.setPadding(ui.dp(6), ui.dp(4), ui.dp(6), 0);
            l.addView(tv, new LinearLayout.LayoutParams(-1, -2));
            f.addView(l, new FrameLayout.LayoutParams(-1, -1));
        }
        GridLayout.LayoutParams lp = new GridLayout.LayoutParams();
        lp.width = ui.dp(bigW()); lp.height = ui.dp(bigH());
        lp.setMargins(ui.dp(bigM()), 0, ui.dp(bigM()), 0);
        f.setLayoutParams(lp);
        f.setContentDescription(label);
        f.setTag(14);
        ui.wire(f, new Runnable() { public void run() { apps.launch(pkg); } }, bigW() >= 150 ? 1.14f : 1.12f);
        f.setLongClickable(true);
        f.setOnLongClickListener(new View.OnLongClickListener() {
            public boolean onLongClick(View v) { host.showTileMenu(index); return true; } });
        return f;
    }

    View addTile() {
        FrameLayout f = ui.roundedFrame(14, ui.theme.p.tileFill);
        GradientDrawable bg = (GradientDrawable) f.getBackground();
        bg.setStroke(Math.max(1, ui.dp(1)), 0x59FFFFFF);
        TextView plus = ui.icon("plus", "+", compactTiles() ? 26 : 34, 0xCCFFFFFF);
        plus.setGravity(Gravity.CENTER); plus.setShadowLayer(0, 0, 0, 0);
        f.addView(plus, new FrameLayout.LayoutParams(-1, -1));
        GridLayout.LayoutParams lp = new GridLayout.LayoutParams();
        lp.width = ui.dp(bigW()); lp.height = ui.dp(bigH());
        lp.setMargins(ui.dp(bigM()), 0, ui.dp(bigM()), 0);
        f.setLayoutParams(lp);
        f.setContentDescription(ui.t("Ajouter une appli", "Add an app"));
        f.setTag(14);
        ui.wire(f, new Runnable() { public void run() { host.pickAppToAdd(); } }, 1.1f);
        f.setLongClickable(true);
        f.setOnLongClickListener(new View.OnLongClickListener() { public boolean onLongClick(View v) { host.showTileMenu(-1); return true; } });
        return f;
    }

    boolean contains(View v) {
        while (v != null) { if (v == view) return true; v = v.getParent() instanceof View ? (View) v.getParent() : null; }
        return false;
    }

    /** Position de v dans la rangee du dock (tuiles + case « + »), ou -1. */
    int indexOf(View v) {
        ViewGroup g = view.getChildCount() > 0 ? (ViewGroup) view.getChildAt(0) : null;
        return g == null || v.getParent() != g ? -1 : g.indexOfChild(v);
    }

    /** Element numero i de la rangee (borne), ou null si le dock est vide. */
    View tileAt(int i) {
        ViewGroup g = view.getChildCount() > 0 ? (ViewGroup) view.getChildAt(0) : null;
        return g == null || g.getChildCount() == 0 ? null : g.getChildAt(Math.max(0, Math.min(i, g.getChildCount() - 1)));
    }
}
