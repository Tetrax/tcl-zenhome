use thiserror::Error;

#[derive(Debug, Error)]
pub enum Erreur {
    #[error("plateforme non prise en charge")]
    PlateformeNonSupportee,
    #[error("URL refusée : {0}")]
    UrlRefusee(String),
    #[error("échec réseau : {0}")]
    Reseau(String),
    #[error("réponse HTTP inattendue ({0})")]
    StatutHttp(u16),
    #[error("réponse trop volumineuse")]
    TropGros,
    #[error("empreinte SHA-256 différente de celle attendue")]
    EmpreinteInvalide,
    #[error("archive refusée : {0}")]
    ArchiveInvalide(String),
    #[error("réponse illisible : {0}")]
    ReponseInvalide(String),
    #[error("nom de ville vide")]
    VilleVide,
    #[error("découverte mDNS : {0}")]
    Mdns(String),
    #[error("erreur de fichier : {0}")]
    Io(#[from] std::io::Error),
}
