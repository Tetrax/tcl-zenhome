//! Liste annotée des applis à alléger (tv/debloat-mibox4.txt), lue avec les règles de `charger_liste` du kit.
//!
//! Format d'une ligne : `paquet  # [balise] Libellé : explication`.

/// Contenu de la liste, compilé dans l'appli (chemin relatif à ce fichier : racine du dépôt).
const LISTE_MIBOX4: &str = include_str!("../../../../tv/debloat-mibox4.txt");

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Balise {
    Aucune,
    Option,
    Garder,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Recommandation {
    Desactiver,
    Garder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entree {
    pub paquet: String,
    pub balise: Balise,
    pub libelle: String,
    /// Texte après « : », première lettre en majuscule ; vide s'il n'y en a pas.
    pub explication: String,
}

impl Entree {
    /// Sans balise et `[option]` : désactiver ; `[option:garder]` : garder.
    pub fn recommandation(&self) -> Recommandation {
        if self.balise == Balise::Garder { Recommandation::Garder } else { Recommandation::Desactiver }
    }
}

#[derive(Debug, PartialEq)]
pub struct ListeVide;

/// `RE_PAQUET` de tv/lib.sh, utilisé par `charger_liste` : `^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$`.
/// Un peu plus souple que `transport::paquet_valide` (segments suivants pouvant commencer par un chiffre) :
/// un paquet accepté ici mais refusé par le transport échouera à l'envoi, jamais en silence.
fn paquet_de_liste(s: &str) -> bool {
    let mut segments = s.split('.');
    let Some(premier) = segments.next() else { return false };
    let mut c = premier.chars();
    if !c.next().is_some_and(|f| f.is_ascii_alphabetic()) || !c.all(|x| x.is_ascii_alphanumeric() || x == '_') {
        return false;
    }
    let mut n = 0;
    for seg in segments {
        n += 1;
        if seg.is_empty() || !seg.chars().all(|x| x.is_ascii_alphanumeric() || x == '_') {
            return false;
        }
    }
    n > 0
}

/// Mêmes règles que `charger_liste` : lignes sans `#` ignorées, paquet = texte avant le premier `#` sans aucun blanc
/// (vide ou invalide : ligne ignorée), commentaire privé d'un espace de tête, balise `[option:garder]` ou `[option]`,
/// libellé = texte avant le premier « : », explication = texte après, première lettre en majuscule.
pub fn analyser(texte: &str) -> Result<Vec<Entree>, ListeVide> {
    let mut liste = Vec::new();
    for ligne in texte.lines() {
        let Some((avant, apres)) = ligne.split_once('#') else { continue };
        let paquet: String = avant.chars().filter(|c| !c.is_whitespace()).collect();
        if paquet.is_empty() || !paquet_de_liste(&paquet) {
            continue;
        }
        let mut c = apres.strip_prefix(' ').unwrap_or(apres);
        let mut balise = Balise::Aucune;
        if let Some(reste) = c.strip_prefix("[option:garder]") {
            (balise, c) = (Balise::Garder, reste);
        } else if let Some(reste) = c.strip_prefix("[option]") {
            (balise, c) = (Balise::Option, reste);
        }
        let c = c.strip_prefix(' ').unwrap_or(c);
        let (libelle, explication) = match c.split_once(" : ") {
            Some((l, e)) => {
                let mut ch = e.chars();
                (l, ch.next().map(|p| p.to_ascii_uppercase().to_string() + ch.as_str()).unwrap_or_default())
            }
            None => (c, String::new()),
        };
        liste.push(Entree { paquet, balise, libelle: libelle.to_string(), explication });
    }
    if liste.is_empty() { Err(ListeVide) } else { Ok(liste) }
}

/// La liste de la Mi Box 4 embarquée dans l'appli.
pub fn liste_mibox4() -> Vec<Entree> {
    analyser(LISTE_MIBOX4).expect("tv/debloat-mibox4.txt est vide ou illisible")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn liste_integree_24_paquets() {
        let l = liste_mibox4();
        assert_eq!(l.len(), 24);
        let n = |b| l.iter().filter(|e| e.balise == b).count();
        assert_eq!((n(Balise::Aucune), n(Balise::Option), n(Balise::Garder)), (11, 10, 3));
        let doublons = l.iter().enumerate().filter(|(i, e)| l[..*i].iter().any(|f| f.paquet == e.paquet)).count();
        assert_eq!(doublons, 0);
    }

    #[test]
    fn entrees_connues() {
        let l = liste_mibox4();
        let e = |p: &str| l.iter().find(|e| e.paquet == p).unwrap().clone();
        let a = e("tv.alphonso.alphonso_eula");
        assert_eq!((a.balise, a.libelle.as_str()), (Balise::Aucune, "Alphonso"));
        assert!(a.explication.starts_with("Reconnaît ce que vous regardez"));
        let k = e("com.google.android.katniss");
        assert_eq!((k.balise, k.libelle.as_str()), (Balise::Garder, "Recherche vocale"));
        assert_eq!(k.recommandation(), Recommandation::Garder);
        let t = e("com.google.android.tts");
        assert_eq!((t.balise, t.recommandation()), (Balise::Option, Recommandation::Desactiver));
        assert_eq!(a.recommandation(), Recommandation::Desactiver);
        assert_eq!(e("android.autoinstalls.config.xioami.mibox3").libelle, "Installations automatiques");
    }

    #[test]
    fn les_paquets_proteges_ne_sont_pas_dans_la_liste() {
        let l = liste_mibox4();
        for p in ["com.google.android.tvlauncher", "com.google.android.tungsten.setupwraith", "tv.zenhome"] {
            assert!(l.iter().all(|e| e.paquet != p), "{p}");
        }
        assert!(l.iter().all(|e| crate::moteur::transport::paquet_valide(&e.paquet)));
    }

    #[test]
    fn lignes_invalides_refusees() {
        let texte = "\
# commentaire seul
pas de diese
 # sans paquet
paquet_sans_point # [option] X : y
1abc.def # [option] chiffre en tête : y
a.b-c # [option] tiret : y
a..b # [option] segment vide : y
a.b. # [option] point final : y
a.b$(id) # [option] injection : y
bon.paquet  # Tel quel : explication ici
";
        let l = analyser(texte).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].paquet, "bon.paquet");
        assert_eq!(analyser("# rien\n\n"), Err(ListeVide));
        assert_eq!(analyser(""), Err(ListeVide));
    }

    #[test]
    fn balises_libelles_explications() {
        let l = analyser(
            "a.b # [option:garder] Un : le texte : avec deux points\n\
             c.d # [option]Collée sans espace\n\
             e.f#[option:garder]    espaces  : e\n\
             g.h # [inconnue] Libellé : x\n\
             i.j # \n\
             k.l # Sans explication\n\
             m.n # [option] Libellé : ébauche\n",
        )
        .unwrap();
        assert_eq!((l[0].balise, l[0].libelle.as_str(), l[0].explication.as_str()), (Balise::Garder, "Un", "Le texte : avec deux points"));
        assert_eq!((l[1].balise, l[1].libelle.as_str(), l[1].explication.as_str()), (Balise::Option, "Collée sans espace", ""));
        // un seul espace est retiré avant et après la balise
        assert_eq!((l[2].balise, l[2].libelle.as_str(), l[2].explication.as_str()), (Balise::Garder, "   espaces ", "E"));
        assert_eq!((l[3].balise, l[3].libelle.as_str()), (Balise::Aucune, "[inconnue] Libellé"));
        assert_eq!((l[4].libelle.as_str(), l[4].explication.as_str()), ("", ""));
        assert_eq!((l[5].libelle.as_str(), l[5].explication.as_str()), ("Sans explication", ""));
        // seule une lettre ASCII est mise en majuscule (comme `tr` en locale C)
        assert_eq!(l[6].explication, "ébauche");
    }

    /// Parité avec le kit : le vrai `charger_liste` de `kit/Zen Home.command` est extrait et exécuté par bash sur la même liste.
    #[test]
    fn parite_avec_charger_liste_du_kit() {
        let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let kit = std::fs::read_to_string(racine.join("kit/Zen Home.command")).expect("kit/Zen Home.command");
        let debut = kit.find("charger_liste() {").expect("charger_liste dans le kit");
        let fonction = &kit[debut..];
        let fonction = &fonction[..fonction.find("\n}\n").expect("fin de charger_liste") + 3];
        let lib = std::fs::read_to_string(racine.join("tv/lib.sh")).unwrap();
        let re = lib.lines().find(|l| l.starts_with("RE_PAQUET=")).expect("RE_PAQUET dans lib.sh");
        let script = format!(
            "{re}\nTV_DIR=\"$1\"; DEBLOAT=debloat-mibox4.txt\nD_PKG=(); D_TAG=(); D_LAB=(); D_EXP=()\n\
             erreur() {{ echo \"$1\" >&2; exit 1; }}\n{fonction}\ncharger_liste\n\
             for ((i = 0; i < ${{#D_PKG[@]}}; i++)); do printf '%s|%s|%s|%s\\n' \"${{D_PKG[i]}}\" \"${{D_TAG[i]}}\" \"${{D_LAB[i]}}\" \"${{D_EXP[i]}}\"; done\n"
        );
        let Ok(sortie) = Command::new("bash").arg("-c").arg(&script).arg("bash").arg(racine.join("tv")).output() else {
            eprintln!("bash absent : test de parité ignoré");
            return;
        };
        assert!(sortie.status.success(), "bash : {}", String::from_utf8_lossy(&sortie.stderr));
        let attendu: Vec<String> = String::from_utf8(sortie.stdout).unwrap().lines().map(String::from).collect();
        let obtenu: Vec<String> = liste_mibox4()
            .iter()
            .map(|e| {
                let tag = match e.balise {
                    Balise::Aucune => "",
                    Balise::Option => "option",
                    Balise::Garder => "garder",
                };
                format!("{}|{}|{}|{}", e.paquet, tag, e.libelle, e.explication)
            })
            .collect();
        assert_eq!(attendu.len(), 24);
        assert_eq!(obtenu, attendu);
    }
}
