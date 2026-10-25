package tv.zenhome;

import java.util.ArrayList;
import java.util.List;

/**
 * Menus lateraux (MenuPanel) : lignes (type, libelle, valeur, icone), geometrie de la surbrillance, navigation bouclee et pile de
 * sous-menus (sans android.*). Hauteurs en unites de la maquette 1280.
 */
final class MenuRules {
    private MenuRules() { }

    static final int ACTION = 0, SUBMENU = 1, CHOICE = 2, SEPARATOR = 3, INFO = 4;   // INFO : texte, non focalisable
    static final int ROW_H = 52, APP_H = 46, SEP_H = 13;   // ligne d'appli (avec sous-texte) un peu plus basse : 10 applis tiennent dans le panneau

    /** Une ligne : action (ferme le menu), sous-menu (valeur courante + chevron), choix exclusif (point a droite), separateur. */
    static final class Row {
        final int kind;
        final String label, value, icon;   // icon : nom Phosphor ou ""
        final boolean selected;            // CHOICE : le choix actuel
        final int swatch;                  // CHOICE : couleur de la pastille de gauche (rgb), -1 sans
        final String sub;                  // texte discret sous le libelle (ligne d'appli : « il y a 5 min »)
        Row(int kind, String label, String value, String icon, boolean selected, int swatch, String sub) {
            this.kind = kind; this.label = label; this.value = value; this.icon = icon; this.selected = selected; this.swatch = swatch; this.sub = sub;
        }
    }

    static Row action(String label, String icon) { return new Row(ACTION, label, "", icon, false, -1, ""); }
    static Row submenu(String label, String value, String icon) { return new Row(SUBMENU, label, value, icon, false, -1, ""); }
    static Row choice(String label, boolean selected, int swatch) { return new Row(CHOICE, label, "", "", selected, swatch, ""); }
    static Row separator() { return new Row(SEPARATOR, "", "", "", false, -1, ""); }
    /** Ligne d'appli : l'icone de l'appli (icone « app » = colonne reservee) et un sous-texte. */
    static Row app(String label, String sub) { return new Row(ACTION, label, "", "app", false, -1, sub); }
    static Row info(String text) { return new Row(INFO, text, "", "", false, -1, ""); }

    static boolean focusable(Row r) { return r.kind != SEPARATOR && r.kind != INFO; }

    static int rowHeight(Row r) { return r.kind == SEPARATOR ? SEP_H : (r.sub.isEmpty() ? ROW_H : APP_H); }

    /** Hauteur totale des lignes. */
    static int height(List<Row> rows) {
        int h = 0;
        for (Row r : rows) h += rowHeight(r);
        return h;
    }

    /** Position verticale du haut de la ligne idx (la surbrillance s'y place) ; hors bornes : borne. */
    static int top(List<Row> rows, int idx) {
        int y = 0;
        for (int i = 0; i < idx && i < rows.size(); i++) y += rowHeight(rows.get(i));
        return y;
    }

    /** Ligne qui prend le focus a l'ouverture : le choix actuel s'il y en a un, sinon la premiere ligne focalisable ; -1 s'il n'y en a pas. */
    static int firstFocus(List<Row> rows) {
        for (int i = 0; i < rows.size(); i++) if (rows.get(i).kind == CHOICE && rows.get(i).selected) return i;
        for (int i = 0; i < rows.size(); i++) if (focusable(rows.get(i))) return i;
        return -1;
    }

    /** Ligne focalisable suivante (dir = 1) ou precedente (dir = -1), en bouclant et en sautant les separateurs ; -1 s'il n'y en a pas. */
    static int step(List<Row> rows, int cur, int dir) {
        int n = rows.size();
        for (int k = 1; k <= n; k++) {
            int i = ((cur + dir * k) % n + n) % n;
            if (focusable(rows.get(i))) return i;
        }
        return -1;
    }

    /** Ligne focalisable la plus proche de idx (apres la suppression d'une ligne) : idx borne, sinon la precedente ; -1 s'il n'y en a pas. */
    static int nearestFocus(List<Row> rows, int idx) {
        if (rows.isEmpty()) return -1;
        int i = Math.max(0, Math.min(idx, rows.size() - 1));
        if (focusable(rows.get(i))) return i;
        int p = step(rows, i, -1);
        return p;
    }

    /** Au moins une ligne a une icone : on reserve alors la colonne des icones pour toutes. */
    static boolean hasIcons(List<Row> rows) {
        for (Row r : rows) if (focusable(r) && (!r.icon.isEmpty() || r.swatch >= 0)) return true;
        return false;
    }

    // ---------- listes de choix

    static List<Row> themeRows(String current, int autoRgb, boolean fr) {
        List<Row> l = new ArrayList<>();
        String cur = ThemeRules.normalize(current);
        for (String n : ThemeRules.NAMES) l.add(choice(ThemeRules.label(n, fr), n.equals(cur), ThemeRules.swatch(n, autoRgb)));
        return l;
    }

    static List<Row> layoutRows(int current, boolean fr) {
        List<Row> l = new ArrayList<>();
        for (int n = 2; n <= 10; n++) l.add(choice(n + (fr ? " applis" : " apps"), n == current, -1));
        return l;
    }

    /** chosen : minutes du minuteur actif (0 : desactive). */
    static List<Row> sleepRows(int chosen, boolean fr) {
        List<Row> l = new ArrayList<>();
        for (int m : SleepRules.CHOICES) l.add(choice(SleepRules.choiceLabel(m, fr), m == chosen, -1));
        return l;
    }

    static String help(boolean fr) { return fr ? "◀  Retour        OK  Valider" : "◀  Back        OK  Select"; }

    // ---------- pile de sous-menus

    /** Pile de pages : la racine ne se depile jamais (Retour sur la racine ferme le menu). */
    static final class NavStack<T> {
        private final List<T> l = new ArrayList<>();
        void push(T t) { l.add(t); }
        boolean canPop() { return l.size() > 1; }
        /** Retire la page du dessus (sauf la racine) et renvoie la page affichee ensuite. */
        T pop() { if (canPop()) l.remove(l.size() - 1); return top(); }
        T top() { return l.isEmpty() ? null : l.get(l.size() - 1); }
        int depth() { return l.size(); }
    }
}
