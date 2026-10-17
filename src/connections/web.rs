use crate::error::{AppError, Result};
pub fn validated_url(value: &str) -> Result<url::Url> {
    let invalid = || {
        AppError::Validation("Adresse Web invalide : utilisez une URL http:// ou https:// sans identifiant ni mot de passe.".into())
    };
    if value.len() > 8192 || value.chars().any(char::is_control) || value.contains('\\') {
        return Err(invalid());
    }
    let url = url::Url::parse(value).map_err(|_| invalid())?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid());
    }
    Ok(url)
}
pub fn open(value: &str) -> Result<()> {
    let url = validated_url(value)?;
    webbrowser::open(url.as_str()).map_err(|_| AppError::Operation("Impossible d’ouvrir le navigateur par défaut. Vérifiez les applications par défaut de votre système.".into()))
}
