package tv.zenhome;

import android.content.ActivityNotFoundException;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.drawable.Drawable;
import android.provider.Settings;

/** Lancement des applis, entrees TV, icones et libelles. */
class Apps {
    /** Applis "entrees TV / chaines" selon les marques. */
    static final String[][] KNOWN_INPUTS = {
            {"com.tcl.tv", "com.tcl.tv.TVActivity"},
            {"com.mediatek.wwtv.tvcenter", null},
            {"org.droidtv.playtv", null},
            {"com.sony.dtv.tvx", null},
            {"com.mitv.tvplayer", null},
            {"com.mitv.livetv", null},
            {"com.hisense.tv", null},
            {"com.google.android.tv", null},
            {"com.android.tv", null},
    };

    final Context ctx;
    final Ui ui;

    Apps(Context ctx, Ui ui) { this.ctx = ctx; this.ui = ui; }

    boolean usable(String pkg) {
        try {
            ApplicationInfo ai = ctx.getPackageManager().getApplicationInfo(pkg, 0);
            return ai.enabled && launchIntent(pkg) != null;
        } catch (Exception e) { return false; }
    }

    String[] findInput() {
        for (String[] in : KNOWN_INPUTS) {
            try {
                ApplicationInfo ai = ctx.getPackageManager().getApplicationInfo(in[0], 0);
                if (!ai.enabled) continue;
                if (in[1] != null || launchIntent(in[0]) != null) return in;
            } catch (Exception ignored) { }
        }
        return null;
    }

    void openInput(String[] in) {
        if (in[1] != null) {
            try {
                Intent i = new Intent(); i.setComponent(new ComponentName(in[0], in[1]));
                i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK); ctx.startActivity(i); return;
            } catch (Exception ignored) { }
        }
        launch(in[0]);
    }

    /** Premier ecran du systeme qui gere l'action « voir les entrees » (null s'il n'y en a pas). */
    ComponentName viewInputsTarget() {
        try {
            java.util.List<android.content.pm.ResolveInfo> l = ctx.getPackageManager().queryIntentActivities(
                    new Intent("com.android.tv.action.VIEW_INPUTS"), 0);
            if (l != null) for (android.content.pm.ResolveInfo ri : l) {
                if (ri.activityInfo != null && ri.activityInfo.enabled)
                    return new ComponentName(ri.activityInfo.packageName, ri.activityInfo.name);
            }
        } catch (Exception ignored) { }
        return null;
    }

    /** Menu des entrees du systeme s'il existe, sinon l'appli TV de la marque. */
    void openInputs() {
        Intent vi = new Intent("com.android.tv.action.VIEW_INPUTS").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        ComponentName target = viewInputsTarget();
        if (target != null) {
            // on vise l'écran trouvé : sinon, quand plusieurs applis le gèrent (accueil Google + XGIMI), Android affiche un choix
            try { ctx.startActivity(new Intent(vi).setComponent(target)); return; } catch (Exception ignored) { }
        }
        if (vi.resolveActivity(ctx.getPackageManager()) != null) {
            try { ctx.startActivity(vi); return; } catch (Exception ignored) { }
        }
        String[] in = findInput();
        if (in != null) openInput(in);
        else ui.toast(ui.t("Aucun menu des entrées trouvé sur cette TV", "No inputs menu found on this TV"));
    }

    static final ComponentName TCL_PANEL = new ComponentName("com.android.tv.settings", "com.tcl.settings.ShowWindowService");

    /** Marque TCL et service des panneaux rapides (Image, Son) present : c'est un service exporte de com.android.tv.settings. */
    static boolean tclPanelAvailable(Context ctx) {
        try {
            return KeyService.isTcl() && ctx.getPackageManager().resolveService(new Intent("com.tcl.settings.SHOW_WINDOW").setComponent(TCL_PANEL), 0) != null;
        } catch (Exception e) { return false; }
    }

    /** Demarre le panneau rapide TCL (« picture » ou « sound ») par-dessus l'appli en cours ; faux s'il est refuse ou introuvable. */
    static boolean startTclPanel(Context ctx, String type) {
        if (!tclPanelAvailable(ctx)) return false;
        try {
            return ctx.startService(new Intent("com.tcl.settings.SHOW_WINDOW").setComponent(TCL_PANEL).putExtra("Type", type)) != null;
        } catch (RuntimeException e) { return false; }   // appli en arriere-plan, service refuse...
    }

    /** Panneau TCL, sinon l'ecran Android equivalent (Image : affichage, Son : son), sinon les parametres. Faux si rien ne s'ouvre. */
    static boolean openTclPanel(Context ctx, String type) {
        if (startTclPanel(ctx, type)) return true;
        String[] tries = {"sound".equals(type) ? Settings.ACTION_SOUND_SETTINGS : Settings.ACTION_DISPLAY_SETTINGS, Settings.ACTION_SETTINGS};
        for (String a : tries) {
            try { ctx.startActivity(new Intent(a).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)); return true; } catch (Exception ignored) { }
        }
        return false;
    }

    /** Partage avec KeyService (qui n'a pas de Context applicatif ici, seulement le PackageManager). */
    static Intent launchIntent(PackageManager pm, String pkg) {
        Intent i = pm.getLeanbackLaunchIntentForPackage(pkg);
        if (i == null) i = pm.getLaunchIntentForPackage(pkg);
        return i;
    }

    Intent launchIntent(String pkg) { return launchIntent(ctx.getPackageManager(), pkg); }

    void launch(String pkg) {
        Intent i = launchIntent(pkg);
        if (i == null) { ui.toast(ui.t("Appli introuvable", "App not found")); return; }
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try { ctx.startActivity(i); } catch (ActivityNotFoundException e) { ui.toast(ui.t("Impossible d'ouvrir", "Cannot open")); }
    }

    Drawable pkgIcon(String pkg) {
        try { return ctx.getPackageManager().getApplicationIcon(pkg); } catch (Exception e) { return null; }
    }

    String appLabel(String pkg) {
        try { PackageManager pm = ctx.getPackageManager(); return pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString(); }
        catch (Exception e) { return pkg; }
    }
}
