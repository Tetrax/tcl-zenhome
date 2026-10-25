package tv.zenhome;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;

/** Carrousel d'applis (prefs « carousel » + « carousel_user_set »). */
class Carousel {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final SharedPreferences prefs;
    final HomeConfig config;
    final Host host;
    List<String> pkgs = new ArrayList<>();
    List<View> tiles = new ArrayList<>();
    HorizontalScrollView view;
    LinearLayout row;

    Carousel(Context ctx, Ui ui, Apps apps, SharedPreferences prefs, HomeConfig config, Host host) {
        this.ctx = ctx; this.ui = ui; this.apps = apps; this.prefs = prefs; this.config = config; this.host = host;
    }

    void build(ViewGroup parent) {
        row = new LinearLayout(ctx);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setClipChildren(false); row.setClipToPadding(false);
        row.setPadding(ui.dp(8), ui.dp(14), ui.dp(8), ui.dp(6));
        view = ui.rail(row);
        FrameLayout.LayoutParams lpR = new FrameLayout.LayoutParams(-1, ui.dp(88), Gravity.BOTTOM);
        lpR.setMargins(ui.dp(48), 0, ui.dp(48), 0);
        parent.addView(view, lpR);
        rebuild(-1);
    }

    boolean contains(View v) {
        while (v != null) { if (v == row) return true; v = v.getParent() instanceof View ? (View) v.getParent() : null; }
        return false;
    }

    /** Positionne la rangee juste sous le dock (hors ecran tant que le bloc du bas n'est pas remonte : voir Levels). */
    void place() {
        FrameLayout.LayoutParams a = (FrameLayout.LayoutParams) view.getLayoutParams();
        a.bottomMargin = ui.dp(DockMetrics.carBase()); view.setLayoutParams(a);
    }

    // Carrousel d'applis : par defaut les applis lancables absentes du dock, sinon la liste choisie
    List<String> compute() {
        List<String> out = new ArrayList<>();
        if (prefs.getBoolean("carousel_user_set", false)) {
            for (String p : prefs.getString("carousel", "").split("[,;\\s]+")) {
                p = p.trim();
                if (p.length() > 0 && !out.contains(p) && apps.usable(p)) out.add(p);
            }
            return out;
        }
        PackageManager pm = ctx.getPackageManager();
        final HashMap<String, String> names = new HashMap<>();
        // deja presents en petites tuiles du dock
        List<String> skip = new ArrayList<>(Arrays.asList("com.android.vending", "com.android.tv.settings", "com.android.settings"));
        String[] inp = apps.findInput();
        if (inp != null) skip.add(inp[0]);
        ComponentName vi = apps.viewInputsTarget();
        if (vi != null) skip.add(vi.getPackageName());
        for (String cat : new String[]{Intent.CATEGORY_LEANBACK_LAUNCHER, Intent.CATEGORY_LAUNCHER}) {
            Intent q = new Intent(Intent.ACTION_MAIN); q.addCategory(cat);
            for (ResolveInfo ri : pm.queryIntentActivities(q, 0)) {
                String p = ri.activityInfo.packageName;
                ApplicationInfo ai = ri.activityInfo.applicationInfo;
                boolean bareSystem = (ai.flags & ApplicationInfo.FLAG_SYSTEM) != 0 && (ai.flags & ApplicationInfo.FLAG_UPDATED_SYSTEM_APP) == 0
                        && ri.activityInfo.banner == 0 && ai.banner == 0;   // appli systeme sans banniere leanback
                if (!names.containsKey(p) && !p.equals(ctx.getPackageName()) && !config.dockPkgs.contains(p) && !skip.contains(p) && !bareSystem
                        && !p.startsWith("com.tcl.") && !p.startsWith("com.mediatek."))   // applis TCL / MediaTek (hors personnalisation)
                    names.put(p, ri.loadLabel(pm).toString());
            }
        }
        out.addAll(names.keySet());
        Collections.sort(out, new java.util.Comparator<String>() {
            public int compare(String a, String b) { return names.get(a).compareToIgnoreCase(names.get(b)); } });
        while (out.size() > 40) out.remove(out.size() - 1);   // plafond memoire
        return out;
    }

    void load() { pkgs = compute(); }

    void save() {
        prefs.edit().putString("carousel", TextUtils.join(",", pkgs)).putBoolean("carousel_user_set", true).apply();
    }

    void rebuild(int focusIndex) {
        View before = host.currentFocus();
        int bi = tiles.indexOf(before);
        row.removeAllViews();
        tiles.clear();
        for (int i = 0; i < pkgs.size(); i++) {
            View v = carTile(pkgs.get(i), i);
            row.addView(v); tiles.add(v);
        }
        row.addView(carPlusTile());
        if (focusIndex >= 0) {
            final View target = row.getChildAt(Math.min(focusIndex, row.getChildCount() - 1));
            row.post(new Runnable() { public void run() { target.requestFocus(); } });
        } else host.fixRowFocus(before, bi, row);   // rendu automatique : ne jamais voler le focus
    }

    View carTile(final String pkg, final int index) {
        String label = apps.appLabel(pkg);
        LinearLayout l = new LinearLayout(ctx);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setGravity(Gravity.CENTER);
        GradientDrawable bg = new GradientDrawable(); bg.setCornerRadius(ui.dp(12)); bg.setColor(ui.theme.p.appFill);
        l.setBackground(bg);
        Drawable ic = apps.pkgIcon(pkg);
        if (ic == null) ic = new Icons.GridIcon();
        ImageView iv = new ImageView(ctx); iv.setImageDrawable(ic);
        LinearLayout.LayoutParams il = new LinearLayout.LayoutParams(ui.dp(34), ui.dp(34)); il.gravity = Gravity.CENTER_HORIZONTAL;
        l.addView(iv, il);
        TextView tv = ui.text(label, 10, ui.fMed, 0xFFFFFFFF); tv.setShadowLayer(0, 0, 0, 0);
        tv.setGravity(Gravity.CENTER); tv.setSingleLine(true); tv.setEllipsize(TextUtils.TruncateAt.END);
        tv.setPadding(ui.dp(6), ui.dp(3), ui.dp(6), 0);
        l.addView(tv, new LinearLayout.LayoutParams(-1, -2));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ui.dp(120), ui.dp(68)); lp.setMargins(ui.dp(5), 0, ui.dp(5), 0);
        l.setLayoutParams(lp);
        l.setTag(12);
        l.setContentDescription(label);
        ui.wire(l, new Runnable() { public void run() { apps.launch(pkg); } }, 1.1f);
        l.setLongClickable(true);
        l.setOnLongClickListener(new View.OnLongClickListener() { public boolean onLongClick(View v) { host.showCarMenu(index); return true; } });
        return l;
    }

    View carPlusTile() {
        FrameLayout f = ui.roundedFrame(12, ui.theme.p.tileFill);
        ((GradientDrawable) f.getBackground()).setStroke(Math.max(1, ui.dp(1)), 0x59FFFFFF);
        TextView plus = ui.icon("plus", "+", 26, 0xCCFFFFFF);
        plus.setGravity(Gravity.CENTER); plus.setShadowLayer(0, 0, 0, 0);
        f.addView(plus, new FrameLayout.LayoutParams(-1, -1));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ui.dp(120), ui.dp(68)); lp.setMargins(ui.dp(5), 0, ui.dp(5), 0);
        f.setLayoutParams(lp);
        f.setTag(12);
        f.setContentDescription(ui.t("Ajouter une appli", "Add an app"));
        ui.wire(f, new Runnable() { public void run() { host.pickCarouselApp(); } }, 1.1f);
        f.setLongClickable(true);
        f.setOnLongClickListener(new View.OnLongClickListener() { public boolean onLongClick(View v) { host.showCarMenu(-1); return true; } });
        return f;
    }
}
