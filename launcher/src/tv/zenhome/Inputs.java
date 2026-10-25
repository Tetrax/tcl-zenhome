package tv.zenhome;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.ResolveInfo;
import android.media.tv.TvContract;
import android.media.tv.TvInputInfo;
import android.media.tv.TvInputManager;
import android.graphics.drawable.GradientDrawable;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.animation.DecelerateInterpolator;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;

/** Panneau des entrees TV (HDMI, AV, tuner) via TvInputManager ; « Plus… » garde l'ecran du systeme. */
class Inputs {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final Overlays ovs;
    final Host host;
    View panel;   // overlay ouvert (null sinon)

    Inputs(Context ctx, Ui ui, Apps apps, Overlays ovs, Host host) { this.ctx = ctx; this.ui = ui; this.apps = apps; this.ovs = ovs; this.host = host; }

    /** Cartes en bas de l'ecran (photo assombrie) : une par entree, puis « Plus… » (ecran des entrees du systeme). */
    void show() {
        List<InputRules.Entry> es = list();
        if (es.isEmpty()) { apps.openInputs(); return; }   // rien d'exploitable : ancien comportement
        int cards = InputRules.cardCount(es.size()), w = InputRules.cardW(cards), h = InputRules.cardH(cards), m = InputRules.cardM(cards);
        final FrameLayout ov = new FrameLayout(ctx);
        ov.setBackgroundColor(0x99000000);
        ov.setClickable(true);
        ov.setClipChildren(false);
        LinearLayout wrap = new LinearLayout(ctx);   // titre + bande
        wrap.setOrientation(LinearLayout.VERTICAL);
        wrap.setClipChildren(false);
        LinearLayout head = new LinearLayout(ctx);
        head.setPadding(ui.dp(DockMetrics.DOCK_PAD), 0, ui.dp(DockMetrics.DOCK_PAD), ui.dp(8));
        TextView title = ui.text(ui.t("Entrées TV", "TV inputs"), 13, ui.fMed, 0xCCFFFFFF);
        TextView help = ui.text(ui.t("OK pour basculer", "OK to switch"), 10, ui.fReg, 0x80FFFFFF);
        head.addView(title, new LinearLayout.LayoutParams(0, -2, 1f));
        head.addView(help, new LinearLayout.LayoutParams(-2, -2));
        wrap.addView(head, new LinearLayout.LayoutParams(-1, -2));
        LinearLayout strip = new LinearLayout(ctx);
        strip.setGravity(Gravity.CENTER_VERTICAL);
        strip.setClipChildren(false); strip.setClipToPadding(false);
        strip.setPadding(ui.dp(DockMetrics.DOCK_PAD), ui.dp(18), ui.dp(DockMetrics.DOCK_PAD), ui.dp(18));
        GradientDrawable glass = new GradientDrawable();   // forme du dock, plus opaque : le fond est masque
        glass.setCornerRadius(ui.dp(28)); ui.theme.fill(glass, Theme.STRIP, Math.max(1, ui.dp(0.75f)));
        strip.setBackground(glass);
        wrap.addView(strip, new LinearLayout.LayoutParams(-2, -2));

        View first = null, last = null, focus = null;
        int fi = InputRules.focusIndex(es);
        for (int i = 0; i <= es.size(); i++) {
            final InputRules.Entry e = i < es.size() ? es.get(i) : null;
            Runnable go = e != null ? new Runnable() { public void run() { open(e); } } : new Runnable() { public void run() { apps.openInputs(); } };
            View c = card(ov, e, go, w, h, m);
            strip.addView(c);
            c.setId(View.generateViewId());
            c.setNextFocusUpId(c.getId()); c.setNextFocusDownId(c.getId());   // ◀ ▶ seulement
            if (last != null) { last.setNextFocusRightId(c.getId()); c.setNextFocusLeftId(last.getId()); }
            if (first == null) first = c;
            if (i == fi) focus = c;
            last = c;
        }
        first.setNextFocusLeftId(first.getId()); last.setNextFocusRightId(last.getId());   // bords bloques
        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
        lp.bottomMargin = ui.dp(DockMetrics.DOCK_BOTTOM);   // a la place du dock
        ov.addView(wrap, lp);
        ovs.openOverlay(ov, focus);
        panel = ov;
        host.cover(true);
        wrap.setTranslationY(ui.dp(24));
        wrap.animate().translationY(0).setDuration(200).setInterpolator(new DecelerateInterpolator()).start();
    }

    /** Carte d'une entree (e) ou carte « Plus… » (e == null) : icone en haut a gauche, libelle et etat en bas. */
    View card(final View ov, InputRules.Entry e, final Runnable go, int w, int h, int m) {
        boolean small = w < 110;
        FrameLayout f = ui.roundedFrame(14, ui.theme.p.cardFill);
        ImageView ic = new ImageView(ctx);
        Icons.GlyphIcon gi = Icons.GlyphIcon.make(ui.pReg, ui.pFill, IconRules.glyph(e == null ? "squares-four" : InputRules.iconName(e.type, e.label)));
        ic.setImageDrawable(gi != null ? gi : new Icons.GridIcon());   // repli sans police
        int is = small ? 18 : 28;
        FrameLayout.LayoutParams li = new FrameLayout.LayoutParams(ui.dp(is), ui.dp(is), Gravity.TOP | Gravity.START);
        li.setMargins(ui.dp(10), ui.dp(small ? 6 : 10), 0, 0);
        f.addView(ic, li);
        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setPadding(ui.dp(10), 0, ui.dp(8), ui.dp(small ? 6 : 9));
        TextView label = ui.text(e == null ? ui.t("Plus…", "More…") : e.label, small ? 9 : 11, ui.fMed, 0xFFFFFFFF);
        label.setSingleLine(true); label.setEllipsize(TextUtils.TruncateAt.END);
        col.addView(label, new LinearLayout.LayoutParams(-1, -2));
        if (e != null) {
            LinearLayout st = new LinearLayout(ctx);
            st.setGravity(Gravity.CENTER_VERTICAL);
            View dot = new View(ctx);
            GradientDrawable dg = new GradientDrawable(); dg.setShape(GradientDrawable.OVAL); dg.setColor(InputRules.stateColor(e.state));
            dot.setBackground(dg);
            LinearLayout.LayoutParams ld = new LinearLayout.LayoutParams(ui.dp(6), ui.dp(6)); ld.rightMargin = ui.dp(5);
            st.addView(dot, ld);
            TextView stt = ui.text(InputRules.stateText(e.state, ui.fr), small ? 8 : 9, ui.fReg, 0xCCFFFFFF);
            stt.setSingleLine(true); stt.setEllipsize(TextUtils.TruncateAt.END);
            st.addView(stt);
            LinearLayout.LayoutParams ls = new LinearLayout.LayoutParams(-1, -2); ls.topMargin = ui.dp(2);
            col.addView(st, ls);
            f.setAlpha(InputRules.cardAlpha(e.state));
        }
        f.addView(col, new FrameLayout.LayoutParams(-1, -2, Gravity.BOTTOM));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ui.dp(w), ui.dp(h));
        lp.setMargins(ui.dp(m), 0, ui.dp(m), 0);
        f.setLayoutParams(lp);
        f.setContentDescription(e == null ? ui.t("Plus…", "More…") : e.label);
        f.setTag(14);
        ui.wire(f, new Runnable() { public void run() { ovs.closeThen(ov, go); } }, w >= 150 ? 1.14f : 1.12f);
        return f;
    }

    /** Entrees visibles, triees ; liste vide si le service n'existe pas ou en cas d'erreur. */
    List<InputRules.Entry> list() {
        List<InputRules.Entry> out = new ArrayList<>();
        try {
            TvInputManager tm = (TvInputManager) ctx.getSystemService(Context.TV_INPUT_SERVICE);
            if (tm == null) return out;
            List<TvInputInfo> all = tm.getTvInputList();
            HashSet<String> hw = new HashSet<>();   // paquets qui fournissent des entrees materielles
            for (TvInputInfo i : all) if (i.isPassthroughInput()) hw.add(i.getServiceInfo().packageName);
            for (TvInputInfo i : all) {
                if (!InputRules.keep(i.isPassthroughInput(), i.getType(), hw.contains(i.getServiceInfo().packageName))) continue;
                if (i.isHidden(ctx)) continue;
                out.add(InputRules.entry(i.getId(), i.loadCustomLabel(ctx), i.loadLabel(ctx), i.getType(), tm.getInputState(i.getId()), ui.fr));
            }
        } catch (Throwable e) { out.clear(); }
        return InputRules.sort(out);
    }

    void open(InputRules.Entry e) {
        if (e.type == InputRules.TYPE_TUNER) {
            // pas de lecture de chaine « passthrough » : la liste des chaines si une appli systeme sait l'afficher, sinon l'appli TV de la marque
            String[] in = apps.findInput();
            if (!start(new Intent(Intent.ACTION_VIEW, TvContract.buildChannelsUriForInput(e.id)))) {
                if (in != null) apps.openInput(in); else apps.openInputs();
            }
            return;
        }
        if (!start(new Intent(Intent.ACTION_VIEW, TvContract.buildChannelUriForPassthroughInput(e.id)))) apps.openInputs();
    }

    /**
     * Lance l'intent sur une appli SYSTEME seulement (FLAG_SYSTEM), en priorite celle de la marque ; jamais d'intent implicite
     * vers une appli tierce. Renvoie faux si aucune appli systeme ne le gere ou si le lancement echoue.
     */
    boolean start(Intent i) {
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try {
            List<ResolveInfo> l = ctx.getPackageManager().queryIntentActivities(i, 0);
            if (l == null) return false;
            ResolveInfo pick = null;
            ComponentName vi = apps.viewInputsTarget();
            String[] in = apps.findInput();
            for (ResolveInfo ri : l) {
                if (ri.activityInfo == null || ri.activityInfo.applicationInfo == null
                        || (ri.activityInfo.applicationInfo.flags & ApplicationInfo.FLAG_SYSTEM) == 0) continue;
                String p = ri.activityInfo.packageName;
                if (pick == null) pick = ri;
                if ((in != null && p.equals(in[0])) || (vi != null && p.equals(vi.getPackageName()))) { pick = ri; break; }
            }
            if (pick == null) return false;
            i.setComponent(new ComponentName(pick.activityInfo.packageName, pick.activityInfo.name));
            ctx.startActivity(i);
            return true;
        } catch (Exception e) { return false; }
    }
}
