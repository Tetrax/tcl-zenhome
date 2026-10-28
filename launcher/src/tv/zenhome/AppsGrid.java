package tv.zenhome;

import android.content.Context;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.widget.FrameLayout;
import android.widget.GridLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/** Liste de toutes les applis (ouvre l'appli) ou selecteur d'appli (renvoie le choix). */
class AppsGrid {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final Overlays ovs;
    final Host host;

    AppsGrid(Context ctx, Ui ui, Apps apps, Overlays ovs, Host host) {
        this.ctx = ctx; this.ui = ui; this.apps = apps; this.ovs = ovs; this.host = host;
    }

    interface Picker { void picked(String pkg); }

    /** Liste de toutes les applis. picker == null : ouvre l'appli ; sinon renvoie l'appli choisie. */
    void showApps(final Picker picker) {
        final PackageManager pm = ctx.getPackageManager();
        List<ResolveInfo> list = new ArrayList<>(Apps.launchable(ctx).values());
        Collections.sort(list, new java.util.Comparator<ResolveInfo>() {
            public int compare(ResolveInfo a, ResolveInfo b) { return a.loadLabel(pm).toString().compareToIgnoreCase(b.loadLabel(pm).toString()); } });
        final FrameLayout ov = new FrameLayout(ctx);
        ov.setBackgroundColor(0xF0101216);
        ov.setClickable(true);
        ScrollView sv = new ScrollView(ctx);
        sv.setClipChildren(false);
        GridLayout g = new GridLayout(ctx);
        g.setColumnCount(5);
        g.setPadding(ui.dp(60), ui.dp(90), ui.dp(60), ui.dp(60));
        g.setClipChildren(false); g.setClipToPadding(false);
        View first = null;
        for (ResolveInfo ri : list) {
            final String pkg = ri.activityInfo.packageName;
            LinearLayout cell = new LinearLayout(ctx);
            cell.setOrientation(LinearLayout.VERTICAL); cell.setGravity(Gravity.CENTER);
            Drawable ban = ri.activityInfo.loadBanner(pm);
            if (ban == null) ban = ri.loadIcon(pm);
            ImageView iv = new ImageView(ctx); iv.setImageDrawable(ban); iv.setScaleType(ImageView.ScaleType.FIT_CENTER);
            cell.addView(iv, new LinearLayout.LayoutParams(ui.dp(150), ui.dp(84)));
            TextView tv = ui.text(ri.loadLabel(pm).toString(), 11, ui.fReg, 0xFFFFFFFF);
            tv.setPadding(0, ui.dp(6), 0, 0); tv.setSingleLine(true); tv.setGravity(Gravity.CENTER); tv.setEllipsize(TextUtils.TruncateAt.END);
            cell.addView(tv, new LinearLayout.LayoutParams(ui.dp(150), -2));
            GridLayout.LayoutParams lp = new GridLayout.LayoutParams(); lp.setMargins(ui.dp(10), ui.dp(10), ui.dp(10), ui.dp(10));
            cell.setPadding(ui.dp(8), ui.dp(8), ui.dp(8), ui.dp(8));
            GradientDrawable bg = new GradientDrawable(); bg.setCornerRadius(ui.dp(14)); bg.setColor(0x14FFFFFF); cell.setBackground(bg);
            cell.setTag(14);
            ui.wire(cell, new Runnable() { public void run() {
                ovs.closeOverlay(ov, picker == null);
                if (picker != null) picker.picked(pkg); else apps.launch(pkg); } }, 1.08f);
            if (picker == null) {
                cell.setLongClickable(true);
                cell.setOnLongClickListener(new View.OnLongClickListener() { public boolean onLongClick(View v) {
                    host.onAppLongPress(pkg, ov); return true; } });
            }
            g.addView(cell, lp);
            if (first == null) first = cell;
        }
        TextView hdr = ui.text(picker == null ? ui.t("Toutes les applications", "All apps") : ui.t("Choisissez une appli", "Choose an app"), 22, ui.fMed, 0xFFFFFFFF);
        FrameLayout.LayoutParams lh = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.START);
        lh.setMargins(ui.dp(70), ui.dp(30), 0, 0);
        if (picker == null) {
            TextView tip = ui.text(ui.t("Appui long sur OK : ajouter au dock", "Long-press OK: add to dock"), 11, ui.fReg, 0x99FFFFFF);
            FrameLayout.LayoutParams lt = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.END);
            lt.setMargins(0, ui.dp(40), ui.dp(70), 0);
            ov.addView(tip, lt);
        }
        sv.addView(g);
        ov.addView(sv, 0, new FrameLayout.LayoutParams(-1, -1));
        ov.addView(hdr, lh);
        ovs.openOverlay(ov, first);
    }
}
