package tv.zenhome;

import android.content.Context;
import android.content.Intent;
import android.provider.Settings;

/** Menu lateral « Reglages » : panneaux rapides TCL Image et Son, puis tous les parametres Android. */
class SettingsMenu {
    final Context ctx;
    final Ui ui;
    final Overlays ovs;

    SettingsMenu(Context ctx, Ui ui, Overlays ovs) { this.ctx = ctx; this.ui = ui; this.ovs = ovs; }

    void show() {
        ovs.showMenu(new MenuPanel.Source() { public MenuPanel.Page build() {
            MenuPanel.Page p = new MenuPanel.Page(ui.t("Réglages", "Settings"), "");
            p.add(MenuRules.action(ui.t("Image", "Picture"), "monitor"), panel("picture"));
            p.add(MenuRules.action(ui.t("Son", "Sound"), "speaker-high"), panel("sound"));
            p.sep();
            p.add(MenuRules.action(ui.t("Tous les paramètres", "All settings"), "gear-six"), new Runnable() { public void run() {
                try { ctx.startActivity(new Intent(Settings.ACTION_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)); }
                catch (Exception e) { ui.toast(ui.t("Réglages indisponibles", "Settings unavailable")); } } });
            return p;
        } });
    }

    /** Le menu est deja ferme quand l'action tourne (MenuPanel). */
    Runnable panel(final String type) {
        return new Runnable() { public void run() {
            if (!Apps.openTclPanel(ctx, type)) ui.toast(ui.t("Réglages indisponibles", "Settings unavailable"));
        } };
    }
}
