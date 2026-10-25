package tv.zenhome;

import android.app.AlertDialog;
import android.content.Context;
import android.graphics.drawable.GradientDrawable;
import android.text.InputType;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Edition de l'accueil : mode edition (raccourci crayon, ou touche Menu hors d'une tuile), deplacement de tuiles (dock / carrousel),
 * menus des tuiles, nombre d'applis, titre, selecteurs d'applis.
 * Editor modifie le dock et le carrousel : il les connait directement (ecart a « les modules ne se connaissent pas »,
 * sinon Host aurait une dizaine de methodes de plus). Le Context recu est celui de l'activite : AlertDialog l'exige.
 */
class Editor {
    final Context ctx;
    final Ui ui;
    final Apps apps;
    final AppsGrid grid;
    final Overlays ovs;
    final HomeConfig config;
    final Dock dock;
    final Carousel carousel;
    final Host host;
    TextView hintView;
    int moveIndex = -1;           // >= 0 : appli en cours de deplacement
    boolean editMode = false;     // mode edition (raccourci crayon, ou touche Menu hors d'une tuile) : OK sur une appli ouvre son menu
    boolean moveCar = false;      // le deplacement en cours concerne le carrousel (moveIndex = position dedans)

    Editor(Context ctx, Ui ui, Apps apps, AppsGrid grid, Overlays ovs, HomeConfig config, Dock dock, Carousel carousel, Host host) {
        this.ctx = ctx; this.ui = ui; this.apps = apps; this.grid = grid; this.ovs = ovs; this.config = config;
        this.dock = dock; this.carousel = carousel; this.host = host;
    }

    void buildHint(ViewGroup parent) {
        // Indication pendant le deplacement
        hintView = ui.text("", 13, ui.fMed, 0xFFFFFFFF);
        GradientDrawable hb = new GradientDrawable(); hb.setCornerRadius(ui.dp(16)); hb.setColor(0xB3101216);
        hintView.setBackground(hb);
        hintView.setPadding(ui.dp(16), ui.dp(8), ui.dp(16), ui.dp(8));
        hintView.setVisibility(View.GONE);
        FrameLayout.LayoutParams lpH = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
        lpH.setMargins(0, 0, 0, ui.dp(DockMetrics.layoutHint(config.layout)));
        parent.addView(hintView, lpH);
    }

    /** Repositionne l'indication au-dessus du dock (apres chaque reconstruction du dock). */
    void placeHint() {
        FrameLayout.LayoutParams lpH = (FrameLayout.LayoutParams) hintView.getLayoutParams();
        if (lpH != null) { lpH.bottomMargin = ui.dp(DockMetrics.layoutHint(config.layout)); hintView.setLayoutParams(lpH); }
    }

    /** Touche pendant un deplacement (appele pour ACTION_DOWN seulement) : toujours consommee. */
    boolean onMoveKey(KeyEvent e) {
        int c = e.getKeyCode();
        int target = moveIndex;
        if (c == KeyEvent.KEYCODE_DPAD_LEFT) target = moveIndex - 1;
        else if (c == KeyEvent.KEYCODE_DPAD_RIGHT) target = moveIndex + 1;
        else if (c == KeyEvent.KEYCODE_DPAD_CENTER || c == KeyEvent.KEYCODE_ENTER || c == KeyEvent.KEYCODE_BACK
                || c == KeyEvent.KEYCODE_NUMPAD_ENTER || c == KeyEvent.KEYCODE_BUTTON_A) {
            endMove(); return true;
        } else return true;
        List<String> mv = moveCar ? carousel.pkgs : config.dockPkgs;
        if (target >= 0 && target < mv.size() && target != moveIndex) {
            Collections.swap(mv, moveIndex, target);
            moveIndex = target;
            if (moveCar) carousel.rebuild(target); else dock.rebuild(target);
        }
        return true;
    }

    /** Appui long sur une appli de la liste « Toutes les applis » : menu Ouvrir / Ajouter au dock. */
    void onAppLongPress(final String pkg, final View ov) {
        MenuPanel.Page p = new MenuPanel.Page(apps.appLabel(pkg), "");
        p.add(MenuRules.action(ui.t("Ouvrir", "Open"), "arrow-square-out"), new Runnable() { public void run() { ovs.closeOverlay(ov, false); apps.launch(pkg); } });
        if (!config.dockPkgs.contains(pkg) && config.dockPkgs.size() < config.layout) {
            p.add(MenuRules.action(ui.t("Ajouter au dock", "Add to dock"), "plus"), new Runnable() { public void run() {
                ovs.closeOverlay(ov, false); config.dockPkgs.add(pkg); config.save(); dock.rebuild(config.dockPkgs.indexOf(pkg)); } });
        } else if (!config.dockPkgs.contains(pkg)) {
            p.add(MenuRules.action(ui.t("Dock plein : remplacer une appli…", "Dock full: replace an app…"), "swap"), new Runnable() { public void run() {
                ovs.closeOverlay(ov, false); chooseSlotFor(pkg); } });
        }
        menu(p);
    }

    /** Menu lateral a une seule page fixe. */
    void menu(final MenuPanel.Page p) {
        ovs.showMenu(new MenuPanel.Source() { public MenuPanel.Page build() { return p; } });
    }

    // Mode edition (appui long sur OK, ou touche Menu)
    void showTileMenu(final int index) {
        if (moveIndex >= 0 || !ovs.isEmpty()) return;
        ovs.showMenu(new MenuPanel.Source() { public MenuPanel.Page build() { return tilePage(index); } });
    }

    /** Menu d'une tuile du dock (index) ou menu general de l'accueil (-1) : actions de la tuile, puis reglages de l'accueil. */
    MenuPanel.Page tilePage(final int index) {
        boolean tile = index >= 0 && index < config.dockPkgs.size();
        MenuPanel.Page p = tile ? new MenuPanel.Page(ui.t("Dock", "Dock"), apps.appLabel(config.dockPkgs.get(index)))
                : new MenuPanel.Page(ui.t("Écran d'accueil", "Home screen"), "");
        if (tile) {
            if (config.dockPkgs.size() > 1) p.add(MenuRules.action(ui.t("Déplacer", "Move"), "arrows-left-right"), new Runnable() { public void run() { startMove(index); } });
            p.add(MenuRules.action(ui.t("Remplacer par une autre appli", "Replace with another app"), "swap"), new Runnable() { public void run() { pickAppToReplace(index); } });
            p.add(MenuRules.action(ui.t("Retirer du dock", "Remove from dock"), "trash"), new Runnable() { public void run() {
                config.dockPkgs.remove(index); config.save(); dock.rebuild(Math.max(0, index - 1)); } });
            p.sep();
        }
        if (config.dockPkgs.size() < config.layout) p.add(MenuRules.action(ui.t("Ajouter une appli", "Add an app"), "plus"), new Runnable() { public void run() { pickAppToAdd(); } });
        p.sub(MenuRules.submenu(ui.t("Nombre d'applis", "Number of apps"), String.valueOf(config.layout), "squares-four"),
                new MenuPanel.Source() { public MenuPanel.Page build() { return layoutPage(); } });
        p.sub(MenuRules.submenu(ui.t("Thème", "Theme"), ThemeRules.label(ui.theme.name, ui.fr), "palette"),
                new MenuPanel.Source() { public MenuPanel.Page build() { return themePage(); } });
        p.add(MenuRules.action(ui.t("Modifier le titre", "Change the title"), "text-aa"), new Runnable() { public void run() { editTitle(); } });
        return p;
    }

    /** Choix du theme de couleur (point sur l'actuel), enregistre dans les prefs « theme ». */
    MenuPanel.Page themePage() {
        List<Runnable> actions = new ArrayList<>();
        for (final String n : ThemeRules.NAMES)
            actions.add(new Runnable() { public void run() { config.prefs.edit().putString("theme", n).apply(); ui.theme.set(n); } });
        return new MenuPanel.Page(ui.t("Thème", "Theme"), "").addAll(MenuRules.themeRows(ui.theme.name, ui.theme.photoTint, ui.fr), actions);
    }

    MenuPanel.Page layoutPage() {
        List<Runnable> actions = new ArrayList<>();
        for (int i = 2; i <= 10; i++) {
            final int n = i;
            actions.add(new Runnable() { public void run() {
                config.layout = n;
                while (config.dockPkgs.size() > config.layout) config.dockPkgs.remove(config.dockPkgs.size() - 1);
                if (config.dockPkgs.size() < config.layout && !config.prefs.getBoolean("dock_user_set", false)) {
                    for (String p : HomeConfig.KNOWN_DOCK) { if (config.dockPkgs.size() >= config.layout) break; if (apps.usable(p) && !config.dockPkgs.contains(p)) config.dockPkgs.add(p); }
                }
                config.save(); dock.rebuild(0); } });
        }
        return new MenuPanel.Page(ui.t("Nombre d'applis", "Number of apps"), "").addAll(MenuRules.layoutRows(config.layout, ui.fr), actions);
    }

    void editTitle() {
        final EditText et = new EditText(ctx);
        et.setSingleLine(true);
        et.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES);
        et.setText("custom".equals(config.titleMode) ? config.titleText : "");
        et.setHint(ui.t("Prénom", "First name"));
        et.setSelection(et.getText().length());
        FrameLayout box = new FrameLayout(ctx);
        box.setPadding(ui.dp(24), ui.dp(8), ui.dp(24), 0);
        box.addView(et);
        new AlertDialog.Builder(ctx, android.R.style.Theme_Material_Dialog_Alert)
                .setTitle(ui.t("Titre de l'écran d'accueil", "Home screen title"))
                .setMessage(ui.t("Votre prénom (affiché après la salutation). Laissez vide pour une salutation simple.", "Your first name (shown after the greeting). Leave empty for a plain greeting."))
                .setView(box)
                .setPositiveButton(ui.t("Valider", "Save"), new android.content.DialogInterface.OnClickListener() {
                    public void onClick(android.content.DialogInterface d, int w) {
                        String s = et.getText().toString().trim();
                        if (s.isEmpty()) { config.titleMode = "device"; config.titleText = ""; } else { config.titleMode = "custom"; config.titleText = s; }
                        config.save(); host.applyTitle(); } })
                .setNeutralButton(ui.t("Masquer la salutation", "Hide greeting"), new android.content.DialogInterface.OnClickListener() {
                    public void onClick(android.content.DialogInterface d, int w) { config.titleMode = "none"; config.save(); host.applyTitle(); } })
                .setNegativeButton(ui.t("Annuler", "Cancel"), null)
                .setOnDismissListener(new android.content.DialogInterface.OnDismissListener() {
                    public void onDismiss(android.content.DialogInterface d) { host.focusDock(); } })
                .show();
    }

    void startEdit() {
        editMode = true;
        hintView.setText(ui.t("OK sur une appli pour la modifier   ·   Retour pour terminer", "OK on an app to edit it   ·   Back to finish"));
        hintView.setVisibility(View.VISIBLE);
        dock.rebuild(0);
    }

    void endEdit() {
        editMode = false;
        hintView.setVisibility(View.GONE);
        int keep = 0;
        View f = host.currentFocus();
        if (f != null && dock.bigTiles.contains(f)) keep = dock.bigTiles.indexOf(f);
        dock.rebuild(keep);
    }

    void startMove(int index) {
        moveIndex = index;
        hintView.setText(ui.t("◀  ▶  pour déplacer   ·   OK pour valider", "◀  ▶  to move   ·   OK to confirm"));
        hintView.setVisibility(View.VISIBLE);
        dock.rebuild(index);
    }

    void endMove() {
        int mi = moveIndex;
        boolean car = moveCar;
        moveIndex = -1; moveCar = false;
        if (editMode) hintView.setText(ui.t("OK sur une appli pour la modifier   ·   Retour pour terminer", "OK on an app to edit it   ·   Back to finish"));
        else hintView.setVisibility(View.GONE);
        if (car) {
            carousel.save();
            carousel.rebuild(mi);
            return;
        }
        config.save();
        int keep = 0;
        View f = host.currentFocus();
        if (f != null && dock.bigTiles.contains(f)) keep = dock.bigTiles.indexOf(f);
        dock.rebuild(keep);
    }

    /** Menu de la tuile ayant le focus : dock ou carrousel. */
    void openTileMenu(View f) {
        if (carousel.contains(f)) showCarMenu(carousel.tiles.indexOf(f)); else showTileMenu(dock.bigTiles.indexOf(f));
    }

    void pickAppToAdd() {
        grid.showApps(new AppsGrid.Picker() { public void picked(String pkg) {
            config.dockPkgs.remove(pkg);
            if (config.dockPkgs.size() >= config.layout) return;
            config.dockPkgs.add(pkg); config.save(); dock.rebuild(config.dockPkgs.indexOf(pkg)); } });
    }

    void pickAppToReplace(final int index) {
        grid.showApps(new AppsGrid.Picker() { public void picked(String pkg) {
            if (index >= config.dockPkgs.size()) return;
            int existing = config.dockPkgs.indexOf(pkg);
            if (existing >= 0 && existing != index) Collections.swap(config.dockPkgs, existing, index);
            else config.dockPkgs.set(index, pkg);
            config.save(); dock.rebuild(index); } });
    }

    void chooseSlotFor(final String pkg) {
        MenuPanel.Page p = new MenuPanel.Page(ui.t("Remplacer une appli", "Replace an app"), apps.appLabel(pkg));
        for (int i = 0; i < config.dockPkgs.size(); i++) {
            final int idx = i;
            p.add(MenuRules.action(ui.t("Remplacer ", "Replace ") + apps.appLabel(config.dockPkgs.get(i)), "swap"),
                    new Runnable() { public void run() { config.dockPkgs.set(idx, pkg); config.save(); dock.rebuild(idx); } });
        }
        menu(p);
    }

    void pickCarouselApp() {
        grid.showApps(new AppsGrid.Picker() { public void picked(String pkg) {
            if (!carousel.pkgs.contains(pkg)) carousel.pkgs.add(pkg);
            carousel.save(); carousel.rebuild(carousel.pkgs.indexOf(pkg)); } });
    }

    void showCarMenu(final int index) {
        if (moveIndex >= 0 || !ovs.isEmpty()) return;
        boolean tile = index >= 0 && index < carousel.pkgs.size();
        MenuPanel.Page p = tile ? new MenuPanel.Page(ui.t("Carrousel", "Carousel"), apps.appLabel(carousel.pkgs.get(index)))
                : new MenuPanel.Page(ui.t("Carrousel d'applis", "App carousel"), "");
        if (tile) {
            if (carousel.pkgs.size() > 1) p.add(MenuRules.action(ui.t("Déplacer", "Move"), "arrows-left-right"), new Runnable() { public void run() { startCarMove(index); } });
            p.add(MenuRules.action(ui.t("Retirer du carrousel", "Remove from carousel"), "trash"), new Runnable() { public void run() {
                carousel.pkgs.remove(index); carousel.save(); carousel.rebuild(Math.max(0, index - 1)); } });
            p.sep();
        }
        p.add(MenuRules.action(ui.t("Ajouter une appli", "Add an app"), "plus"), new Runnable() { public void run() { pickCarouselApp(); } });
        p.add(MenuRules.action(ui.t("Réinitialiser", "Reset"), "arrow-counter-clockwise"), new Runnable() { public void run() {
            config.prefs.edit().remove("carousel").putBoolean("carousel_user_set", false).apply();
            carousel.load(); carousel.rebuild(0); } });
        menu(p);
    }

    void startCarMove(int index) {
        moveIndex = index; moveCar = true;
        hintView.setText(ui.t("◀  ▶  pour déplacer   ·   OK pour valider", "◀  ▶  to move   ·   OK to confirm"));
        hintView.setVisibility(View.VISIBLE);
        carousel.rebuild(index);
    }
}
