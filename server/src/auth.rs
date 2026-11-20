use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::FromRequestParts,
    http::{header, HeaderMap, Method},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use qrcodegen::{QrCode, QrCodeEcc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::{config::ensure_private_file, error::AppError, state::AppState};

const LOGIN_WINDOW: Duration = Duration::from_secs(5 * 60);
const MAX_LOGIN_FAILURES: usize = 8;
const MAX_TOKEN_LENGTH: usize = 4096;

type HmacSha256 = Hmac<Sha256>;
type HmacSha1 = Hmac<Sha1>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub display_name: String,
    pub role: Role,
    #[serde(default = "default_version")]
    pub token_version: u64,
    pub password_hash: String,
    #[serde(default)]
    pub totp_secret: Option<String>,
    #[serde(default)]
    pub totp_enabled: bool,
    pub created_at: String,
}

fn default_version() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub totp_enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TotpSetup {
    pub secret: String,
    pub otpauth_url: String,
    pub qr_svg: String,
}

impl From<&User> for PublicUser {
    fn from(user: &User) -> Self {
        Self {
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            role: user.role,
            totp_enabled: user.totp_enabled,
            created_at: user.created_at.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UsersFile {
    version: u32,
    users: Vec<User>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
    pub role: Role,
    pub session_id: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.is_admin() {
            Ok(())
        } else {
            Err(AppError::forbidden(
                "Din konto har ikke adgang til denne handling.",
            ))
        }
    }
}

struct LoginLimiter {
    failures: HashMap<String, VecDeque<Instant>>,
}

impl LoginLimiter {
    fn new() -> Self {
        Self {
            failures: HashMap::new(),
        }
    }

    fn prune(&mut self, key: &str) {
        let now = Instant::now();
        if let Some(entries) = self.failures.get_mut(key) {
            while entries
                .front()
                .map(|instant| now.duration_since(*instant) > LOGIN_WINDOW)
                .unwrap_or(false)
            {
                entries.pop_front();
            }
            if entries.is_empty() {
                self.failures.remove(key);
            }
        }
    }

    fn check(&mut self, key: &str) -> Result<(), u64> {
        self.prune(key);
        if self.failures.len() > 10_000 {
            self.failures.clear();
        }
        let Some(entries) = self.failures.get(key) else {
            return Ok(());
        };
        if entries.len() < MAX_LOGIN_FAILURES {
            return Ok(());
        }
        let oldest = entries.front().copied().unwrap_or_else(Instant::now);
        let retry = LOGIN_WINDOW
            .saturating_sub(Instant::now().duration_since(oldest))
            .as_secs()
            .max(1);
        Err(retry)
    }

    fn record_failure(&mut self, key: &str) {
        let entries = self.failures.entry(key.to_owned()).or_default();
        entries.push_back(Instant::now());
        while entries.len() > MAX_LOGIN_FAILURES {
            entries.pop_front();
        }
    }

    fn clear(&mut self, key: &str) {
        self.failures.remove(key);
    }
}

pub struct AuthService {
    users_path: PathBuf,
    sessions_path: PathBuf,
    secret: [u8; 32],
    ttl: Duration,
    cookie_secure: bool,
    users: RwLock<Vec<User>>,
    sessions: RwLock<Vec<SessionRecord>>,
    login_limiter: Mutex<LoginLimiter>,
    setup_lock: tokio::sync::Mutex<()>,
    dummy_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionRecord {
    id: String,
    username: String,
    role: Role,
    ip: String,
    user_agent: String,
    created_at: String,
    last_seen_at: String,
    expires_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub username: String,
    pub role: Role,
    pub ip: String,
    pub user_agent: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub expires_at: i64,
    pub current: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    role: Role,
    ver: u64,
    iat: i64,
    exp: i64,
    jti: String,
}

#[derive(Debug)]
struct TokenIdentity {
    username: String,
    role: Role,
    session_id: String,
}

impl AuthService {
    pub fn load(
        users_path: PathBuf,
        secret_path: &Path,
        ttl: Duration,
        cookie_secure: bool,
    ) -> Result<Arc<Self>> {
        let secret = load_or_create_secret(secret_path)?;
        let users = load_users(&users_path)?;
        let sessions_path = users_path.with_file_name("sessions.json");
        let sessions = load_sessions(&sessions_path)?;
        let dummy_hash = hash_password_sync("nordtorrent-dummy-password")
            .context("could not initialize password verifier")?;
        Ok(Arc::new(Self {
            users_path,
            sessions_path,
            secret,
            ttl,
            cookie_secure,
            users: RwLock::new(users),
            sessions: RwLock::new(sessions),
            login_limiter: Mutex::new(LoginLimiter::new()),
            setup_lock: tokio::sync::Mutex::new(()),
            dummy_hash,
        }))
    }

    pub fn setup_required(&self) -> bool {
        self.users
            .read()
            .map(|users| users.is_empty())
            .unwrap_or(false)
    }

    pub fn list_users(&self) -> Vec<PublicUser> {
        self.users
            .read()
            .map(|users| users.iter().map(PublicUser::from).collect())
            .unwrap_or_default()
    }

    pub fn user(&self, username: &str) -> Option<PublicUser> {
        self.users.read().ok().and_then(|users| {
            users
                .iter()
                .find(|user| user.username == username)
                .map(PublicUser::from)
        })
    }

    pub fn ttl_seconds(&self) -> i64 {
        self.ttl.as_secs() as i64
    }

    pub fn setup_totp(&self, username: &str) -> Result<TotpSetup, AppError> {
        let secret = generate_totp_secret();
        let mut users = self
            .users
            .write()
            .map_err(|_| AppError::internal("user store poisoned", "lock poisoned"))?;
        let user = users
            .iter_mut()
            .find(|user| user.username == username)
            .ok_or_else(|| AppError::unauthorized("Ukendt bruger."))?;
        if user.totp_enabled {
            return Err(AppError::conflict(
                "2FA er allerede slået til. Slå det fra først, hvis du vil opsætte det igen.",
            ));
        }
        user.totp_secret = Some(secret.clone());
        user.totp_enabled = false;
        persist_users(&self.users_path, &users)
            .map_err(|err| AppError::internal("could not persist users", err))?;
        let otpauth_url = totp_url(username, &secret);
        let qr = QrCode::encode_text(&otpauth_url, QrCodeEcc::Medium)
            .map_err(|err| AppError::internal("could not create 2FA QR code", err))?;
        let qr_svg = qr_svg(&qr, 4);
        Ok(TotpSetup {
            otpauth_url,
            secret,
            qr_svg,
        })
    }

    pub fn enable_totp(&self, username: &str, code: &str) -> Result<PublicUser, AppError> {
        let user = self
            .users
            .read()
            .ok()
            .and_then(|users| users.iter().find(|user| user.username == username).cloned())
            .ok_or_else(|| AppError::unauthorized("Ukendt bruger."))?;
        let secret = user
            .totp_secret
            .as_deref()
            .ok_or_else(|| AppError::bad_request("Start 2FA-opsætningen først."))?;
        if !verify_totp(secret, code, Utc::now().timestamp()) {
            return Err(AppError::unauthorized("Forkert 2FA-kode."));
        }
        let mut users = self
            .users
            .write()
            .map_err(|_| AppError::internal("user store poisoned", "lock poisoned"))?;
        let stored = users
            .iter_mut()
            .find(|user| user.username == username)
            .ok_or_else(|| AppError::not_found("Ukendt bruger."))?;
        stored.totp_enabled = true;
        let public = PublicUser::from(&*stored);
        persist_users(&self.users_path, &users)
            .map_err(|err| AppError::internal("could not persist users", err))?;
        Ok(public)
    }

    pub async fn disable_totp(
        &self,
        username: &str,
        password: &str,
    ) -> Result<PublicUser, AppError> {
        let user = self
            .users
            .read()
            .ok()
            .and_then(|users| users.iter().find(|user| user.username == username).cloned())
            .ok_or_else(|| AppError::unauthorized("Ukendt bruger."))?;
        let password = password.to_owned();
        let hash = user.password_hash.clone();
        let valid = tokio::task::spawn_blocking(move || verify_password_sync(&password, &hash))
            .await
            .map_err(|err| AppError::internal("password worker failed", err))?;
        if !valid {
            return Err(AppError::unauthorized("Adgangskoden er forkert."));
        }
        let mut users = self
            .users
            .write()
            .map_err(|_| AppError::internal("user store poisoned", "lock poisoned"))?;
        let stored = users
            .iter_mut()
            .find(|user| user.username == username)
            .ok_or_else(|| AppError::not_found("Ukendt bruger."))?;
        stored.totp_secret = None;
        stored.totp_enabled = false;
        let public = PublicUser::from(&*stored);
        persist_users(&self.users_path, &users)
            .map_err(|err| AppError::internal("could not persist users", err))?;
        Ok(public)
    }

    pub fn list_sessions(&self, current_id: &str) -> Vec<SessionInfo> {
        let now = Utc::now().timestamp();
        let mut sessions = self
            .sessions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        sessions.retain(|session| session.expires_at > now);
        let mut result: Vec<SessionInfo> = sessions
            .iter()
            .map(|session| SessionInfo {
                id: session.id.clone(),
                username: session.username.clone(),
                role: session.role,
                ip: session.ip.clone(),
                user_agent: session.user_agent.clone(),
                created_at: session.created_at.clone(),
                last_seen_at: session.last_seen_at.clone(),
                expires_at: session.expires_at,
                current: session.id == current_id,
            })
            .collect();
        result.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        result
    }

    pub fn revoke_session(&self, id: &str) -> Result<(), AppError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AppError::internal("session store poisoned", "lock poisoned"))?;
        let before = sessions.len();
        sessions.retain(|session| session.id != id);
        if sessions.len() == before {
            return Err(AppError::not_found("Sessionen findes ikke."));
        }
        persist_sessions(&self.sessions_path, &sessions)
            .map_err(|err| AppError::internal("could not persist sessions", err))?;
        Ok(())
    }

    fn revoke_user_sessions(&self, username: &str) -> Result<(), AppError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AppError::internal("session store poisoned", "lock poisoned"))?;
        sessions.retain(|session| session.username != username);
        persist_sessions(&self.sessions_path, &sessions)
            .map_err(|err| AppError::internal("could not persist sessions", err))?;
        Ok(())
    }

    pub async fn bootstrap_admin(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<PublicUser>> {
        let _guard = self.setup_lock.lock().await;
        if !self.setup_required() {
            return Ok(None);
        }
        let user = self
            .add_user(username, username, password, Role::Admin)
            .await?;
        Ok(Some(user))
    }

    pub async fn setup(
        &self,
        username: &str,
        display_name: &str,
        password: &str,
    ) -> Result<(PublicUser, String), AppError> {
        let _guard = self.setup_lock.lock().await;
        if !self.setup_required() {
            return Err(AppError::conflict("Opsætningen er allerede gennemført."));
        }
        let user = self
            .add_user(username, display_name, password, Role::Admin)
            .await?;
        let (user, token) = self.authenticate(&user.username, password, "setup").await?;
        Ok((user, token))
    }

    pub async fn add_user(
        &self,
        username: &str,
        display_name: &str,
        password: &str,
        role: Role,
    ) -> Result<PublicUser, AppError> {
        let username = normalize_username(username)?;
        let display_name = normalize_display_name(display_name, &username)?;
        validate_password(password, &username)?;

        let password = password.to_owned();
        let hash = tokio::task::spawn_blocking(move || hash_password_sync(&password))
            .await
            .map_err(|err| AppError::internal("password worker failed", err))?
            .map_err(|err| AppError::internal("password hashing failed", err))?;

        let mut users = self
            .users
            .write()
            .map_err(|_| AppError::internal("user store poisoned", "lock poisoned"))?;
        if users.iter().any(|user| user.username == username) {
            return Err(AppError::conflict("Brugernavnet er allerede i brug."));
        }
        if users.is_empty() && role != Role::Admin {
            return Err(AppError::bad_request(
                "Den første konto skal være administrator.",
            ));
        }
        let user = User {
            username,
            display_name,
            role,
            token_version: 1,
            password_hash: hash,
            totp_secret: None,
            totp_enabled: false,
            created_at: Utc::now().to_rfc3339(),
        };
        users.push(user.clone());
        persist_users(&self.users_path, &users)
            .map_err(|err| AppError::internal("could not persist users", err))?;
        Ok(PublicUser::from(&user))
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
        client_key: &str,
    ) -> Result<(PublicUser, String), AppError> {
        self.authenticate_with_context(username, password, client_key, "", "")
            .await
    }

    pub async fn authenticate_with_context(
        &self,
        username: &str,
        password: &str,
        client_key: &str,
        ip: &str,
        user_agent: &str,
    ) -> Result<(PublicUser, String), AppError> {
        self.authenticate_with_totp_context(
            username, password, client_key, ip, user_agent, None, true,
        )
        .await
    }

    pub async fn authenticate_with_totp_context(
        &self,
        username: &str,
        password: &str,
        client_key: &str,
        ip: &str,
        user_agent: &str,
        totp_code: Option<&str>,
        local: bool,
    ) -> Result<(PublicUser, String), AppError> {
        let username = normalize_username(username)
            .map_err(|_| AppError::unauthorized("Forkert brugernavn eller adgangskode."))?;
        let limiter_key = format!("{client_key}:{username}");
        {
            let mut limiter = self
                .login_limiter
                .lock()
                .map_err(|_| AppError::internal("login limiter poisoned", "lock poisoned"))?;
            if let Err(retry) = limiter.check(&limiter_key) {
                return Err(AppError::too_many_requests(format!(
                    "For mange loginforsøg. Prøv igen om {retry} sekunder."
                )));
            }
        }

        let user = self
            .users
            .read()
            .ok()
            .and_then(|users| users.iter().find(|user| user.username == username).cloned());
        let (hash, exists) = match &user {
            Some(user) => (user.password_hash.clone(), true),
            None => (self.dummy_hash.clone(), false),
        };
        let password = password.to_owned();
        let valid = tokio::task::spawn_blocking(move || verify_password_sync(&password, &hash))
            .await
            .map_err(|err| AppError::internal("password worker failed", err))?;

        if !valid || !exists {
            if let Ok(mut limiter) = self.login_limiter.lock() {
                limiter.record_failure(&limiter_key);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
            return Err(AppError::unauthorized(
                "Forkert brugernavn eller adgangskode.",
            ));
        }

        let user = user.expect("checked exists above");
        if !local && user.totp_enabled {
            let code = totp_code.unwrap_or("").trim();
            if code.is_empty() {
                return Err(AppError::new(
                    axum::http::StatusCode::UNAUTHORIZED,
                    "totp_required",
                    "Indtast koden fra Google Authenticator.",
                ));
            }
            let Some(secret) = user.totp_secret.as_deref() else {
                return Err(AppError::new(
                    axum::http::StatusCode::UNAUTHORIZED,
                    "totp_invalid",
                    "2FA er konfigureret uden en gyldig hemmelighed.",
                ));
            };
            if !verify_totp(secret, code, Utc::now().timestamp()) {
                if let Ok(mut limiter) = self.login_limiter.lock() {
                    limiter.record_failure(&limiter_key);
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
                return Err(AppError::new(
                    axum::http::StatusCode::UNAUTHORIZED,
                    "totp_invalid",
                    "Forkert 2FA-kode.",
                ));
            }
        }

        if let Ok(mut limiter) = self.login_limiter.lock() {
            limiter.clear(&limiter_key);
        }
        let token = self
            .issue_token(&user, ip, user_agent)
            .map_err(|err| AppError::internal("could not issue authentication token", err))?;
        Ok((PublicUser::from(&user), token))
    }

    pub async fn change_password(
        &self,
        username: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<(PublicUser, String), AppError> {
        let user = self
            .users
            .read()
            .ok()
            .and_then(|users| users.iter().find(|user| user.username == username).cloned())
            .ok_or_else(|| AppError::unauthorized("Ukendt bruger."))?;
        let current = current_password.to_owned();
        let hash = user.password_hash.clone();
        let valid = tokio::task::spawn_blocking(move || verify_password_sync(&current, &hash))
            .await
            .map_err(|err| AppError::internal("password worker failed", err))?;
        if !valid {
            return Err(AppError::unauthorized("Nuværende adgangskode er forkert."));
        }
        validate_password(new_password, username)?;

        let new_password = new_password.to_owned();
        let new_hash = tokio::task::spawn_blocking(move || hash_password_sync(&new_password))
            .await
            .map_err(|err| AppError::internal("password worker failed", err))?
            .map_err(|err| AppError::internal("password hashing failed", err))?;

        let mut users = self
            .users
            .write()
            .map_err(|_| AppError::internal("user store poisoned", "lock poisoned"))?;
        let stored = users
            .iter_mut()
            .find(|candidate| candidate.username == username)
            .ok_or_else(|| AppError::not_found("Ukendt bruger."))?;
        stored.password_hash = new_hash;
        stored.token_version = stored.token_version.saturating_add(1);
        let public = PublicUser::from(&*stored);
        self.revoke_user_sessions(username)?;
        let token = self
            .issue_token(stored, "", "")
            .map_err(|err| AppError::internal("could not issue token", err))?;
        persist_users(&self.users_path, &users)
            .map_err(|err| AppError::internal("could not persist users", err))?;
        Ok((public, token))
    }

    pub fn delete_user(&self, username: &str, actor: &str) -> Result<(), AppError> {
        let mut users = self
            .users
            .write()
            .map_err(|_| AppError::internal("user store poisoned", "lock poisoned"))?;
        if username == actor {
            return Err(AppError::bad_request("Du kan ikke slette din egen konto."));
        }
        let Some(index) = users.iter().position(|user| user.username == username) else {
            return Err(AppError::not_found("Ukendt bruger."));
        };
        let removing_admin = users[index].role == Role::Admin;
        let admin_count = users.iter().filter(|u| u.role == Role::Admin).count();
        if removing_admin && admin_count <= 1 {
            return Err(AppError::bad_request(
                "Der skal altid være mindst én administrator.",
            ));
        }
        users.remove(index);
        persist_users(&self.users_path, &users)
            .map_err(|err| AppError::internal("could not persist users", err))?;
        self.revoke_user_sessions(username)?;
        Ok(())
    }

    fn issue_token(&self, user: &User, ip: &str, user_agent: &str) -> Result<String> {
        let now = Utc::now().timestamp();
        let mut jti = [0_u8; 16];
        OsRng.fill_bytes(&mut jti);
        let session_id = hex(&jti);
        let created_at = Utc::now().to_rfc3339();
        let claims = Claims {
            iss: "nordtorrent".to_owned(),
            sub: user.username.clone(),
            role: user.role,
            ver: user.token_version,
            iat: now,
            exp: now + self.ttl_seconds(),
            jti: session_id.clone(),
        };
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
        let unsigned = format!("{header}.{payload}");
        let signature = self.sign(unsigned.as_bytes())?;

        let record = SessionRecord {
            id: session_id,
            username: user.username.clone(),
            role: user.role,
            ip: truncate_metadata(ip, 128),
            user_agent: truncate_metadata(user_agent, 300),
            created_at: created_at.clone(),
            last_seen_at: created_at,
            expires_at: claims.exp,
        };
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
        sessions.push(record);
        // Keep the file bounded even if a deployment has a very long TTL.
        if sessions.len() > 1000 {
            let excess = sessions.len() - 1000;
            sessions.drain(0..excess);
        }
        persist_sessions(&self.sessions_path, &sessions)?;
        Ok(format!("{unsigned}.{signature}"))
    }

    fn validate_token(&self, token: &str) -> Result<TokenIdentity, AppError> {
        if token.len() > MAX_TOKEN_LENGTH {
            return Err(AppError::unauthorized("Ugyldig session."));
        }
        let mut parts = token.split('.');
        let (Some(header), Some(payload), Some(signature), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(AppError::unauthorized("Ugyldig session."));
        };
        let header_json = URL_SAFE_NO_PAD
            .decode(header.as_bytes())
            .map_err(|_| AppError::unauthorized("Ugyldig session."))?;
        let header_value: serde_json::Value = serde_json::from_slice(&header_json)
            .map_err(|_| AppError::unauthorized("Ugyldig session."))?;
        if header_value.get("alg").and_then(|v| v.as_str()) != Some("HS256")
            || header_value.get("typ").and_then(|v| v.as_str()) != Some("JWT")
        {
            return Err(AppError::unauthorized("Ugyldig session."));
        }

        let expected = self
            .sign(format!("{header}.{payload}").as_bytes())
            .map_err(|err| AppError::internal("could not verify token", err))?;
        if expected.as_bytes().ct_eq(signature.as_bytes()).unwrap_u8() != 1 {
            return Err(AppError::unauthorized("Ugyldig session."));
        }

        let claims_json = URL_SAFE_NO_PAD
            .decode(payload.as_bytes())
            .map_err(|_| AppError::unauthorized("Ugyldig session."))?;
        let claims: Claims = serde_json::from_slice(&claims_json)
            .map_err(|_| AppError::unauthorized("Ugyldig session."))?;
        let now = Utc::now().timestamp();
        if claims.iss != "nordtorrent"
            || claims.exp <= now
            || claims.iat > now + 120
            || claims.jti.len() < 16
        {
            return Err(AppError::unauthorized(
                "Sessionen er udløbet. Log ind igen.",
            ));
        }

        let user = self
            .users
            .read()
            .ok()
            .and_then(|users| {
                users
                    .iter()
                    .find(|user| user.username == claims.sub)
                    .cloned()
            })
            .ok_or_else(|| AppError::unauthorized("Sessionen er ikke længere gyldig."))?;
        if user.token_version != claims.ver || user.role != claims.role {
            return Err(AppError::unauthorized(
                "Sessionen er ikke længere gyldig. Log ind igen.",
            ));
        }

        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AppError::internal("session store poisoned", "lock poisoned"))?;
        let Some(session) = sessions
            .iter_mut()
            .find(|session| session.id == claims.jti)
        else {
            return Err(AppError::unauthorized(
                "Sessionen er blevet tilbagekaldt. Log ind igen.",
            ));
        };
        if session.username != user.username || session.role != user.role {
            return Err(AppError::unauthorized("Ugyldig session."));
        }
        session.last_seen_at = Utc::now().to_rfc3339();

        Ok(TokenIdentity {
            username: user.username,
            role: user.role,
            session_id: claims.jti,
        })
    }

    pub fn session_cookie(&self, token: &str) -> String {
        let secure = if self.cookie_secure { "; Secure" } else { "" };
        format!(
            "nord_session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{secure}",
            self.ttl.as_secs()
        )
    }

    pub fn clear_cookie(&self) -> String {
        let secure = if self.cookie_secure { "; Secure" } else { "" };
        format!("nord_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}")
    }

    fn sign(&self, input: &[u8]) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|err| anyhow::anyhow!("invalid HMAC key: {err}"))?;
        mac.update(input);
        Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (token, via_cookie) = extract_token(&parts.headers)
            .ok_or_else(|| AppError::unauthorized("Log ind for at fortsætte."))?;
        let identity = state.auth.validate_token(&token)?;
        let safe_method = matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS);

        if via_cookie && !safe_method {
            let requested_with = parts
                .headers
                .get("x-requested-with")
                .and_then(|value| value.to_str().ok());
            if requested_with != Some("NordTorrent") {
                return Err(AppError::forbidden(
                    "Anmodningen mangler det påkrævede sikkerhedsheader.",
                ));
            }
            if let Some(fetch_site) = parts
                .headers
                .get("sec-fetch-site")
                .and_then(|value| value.to_str().ok())
            {
                if fetch_site != "same-origin" && fetch_site != "none" {
                    return Err(AppError::forbidden("Cross-site anmodning afvist."));
                }
            }
            if !origin_allowed(state, &parts.headers) {
                return Err(AppError::forbidden("Origin er ikke tilladt."));
            }
        }

        Ok(Self {
            username: identity.username,
            role: identity.role,
            session_id: identity.session_id,
        })
    }
}

pub fn origin_allowed(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let origin = origin.trim_end_matches('/');
    if let Some(public_url) = &state.config.public_url {
        if origin.eq_ignore_ascii_case(public_url.trim_end_matches('/')) {
            return true;
        }
    }
    let host = if state.config.trust_proxy {
        headers
            .get("x-forwarded-host")
            .and_then(|v| v.to_str().ok())
            .or_else(|| headers.get(header::HOST).and_then(|v| v.to_str().ok()))
    } else {
        headers.get(header::HOST).and_then(|v| v.to_str().ok())
    };
    let Some(host) = host else {
        return false;
    };
    origin.eq_ignore_ascii_case(&format!("http://{host}"))
        || origin.eq_ignore_ascii_case(&format!("https://{host}"))
}

fn extract_token(headers: &HeaderMap) -> Option<(String, bool)> {
    if let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    {
        if let Some(token) = value.strip_prefix("Bearer ") {
            let token = token.trim();
            if !token.is_empty() {
                return Some((token.to_owned(), false));
            }
        }
    }
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("nord_session=") {
            if !value.is_empty() {
                return Some((value.to_owned(), true));
            }
        }
    }
    None
}

pub fn normalize_username(raw: &str) -> Result<String, AppError> {
    let username = raw.trim().to_ascii_lowercase();
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::bad_request("Brugernavn skal være 3-32 tegn."));
    }
    if !username
        .chars()
        .next()
        .map(|c| c.is_ascii_alphanumeric())
        .unwrap_or(false)
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(AppError::bad_request(
            "Brugernavn må kun indeholde a-z, 0-9, punktum, bindestreg og understregning.",
        ));
    }
    Ok(username)
}

fn normalize_display_name(raw: &str, username: &str) -> Result<String, AppError> {
    let display = raw.trim();
    let display = if display.is_empty() {
        username.to_owned()
    } else {
        display.to_owned()
    };
    if display.chars().count() > 64 || display.chars().any(char::is_control) {
        return Err(AppError::bad_request("Visningsnavn er ugyldigt."));
    }
    Ok(display)
}

pub fn validate_password(password: &str, username: &str) -> Result<(), AppError> {
    let length = password.chars().count();
    if length < 12 {
        return Err(AppError::bad_request(
            "Adgangskoden skal være mindst 12 tegn.",
        ));
    }
    if length > 256 {
        return Err(AppError::bad_request(
            "Adgangskoden må højst være 256 tegn.",
        ));
    }
    if password.trim().is_empty() || password.eq_ignore_ascii_case(username) {
        return Err(AppError::bad_request("Vælg en stærkere adgangskode."));
    }
    Ok(())
}

fn hash_password_sync(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| anyhow::anyhow!("argon2: {err}"))
}

fn verify_password_sync(password: &str, encoded: &str) -> bool {
    let Ok(hash) = PasswordHash::new(encoded) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
}

fn load_sessions(path: &Path) -> Result<Vec<SessionRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("could not read session database {}", path.display()))?;
    let mut sessions: Vec<SessionRecord> = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid session database {}", path.display()))?;
    let now = Utc::now().timestamp();
    sessions.retain(|session| session.expires_at > now && session.id.len() >= 16);
    Ok(sessions)
}

fn persist_sessions(path: &Path, sessions: &[SessionRecord]) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(sessions)?;
    atomic_write(path, &bytes)
}

fn load_users(path: &Path) -> Result<Vec<User>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("could not read user database {}", path.display()))?;
    let parsed: UsersFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid user database {}", path.display()))?;
    if parsed.version != 1 {
        bail!("unsupported user database version {}", parsed.version);
    }
    Ok(parsed.users)
}

fn persist_users(path: &Path, users: &[User]) -> Result<()> {
    let file = UsersFile {
        version: 1,
        users: users.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&file)?;
    atomic_write(path, &bytes)
}

fn load_or_create_secret(path: &Path) -> Result<[u8; 32]> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("could not read secret {}", path.display()))?;
        let decoded = URL_SAFE_NO_PAD
            .decode(raw.trim().as_bytes())
            .context("session secret is not valid base64")?;
        let secret: [u8; 32] = decoded
            .try_into()
            .map_err(|_| anyhow::anyhow!("session secret has wrong length"))?;
        return Ok(secret);
    }
    let mut secret = [0_u8; 32];
    OsRng.fill_bytes(&mut secret);
    let encoded = URL_SAFE_NO_PAD.encode(secret);
    atomic_write(path, encoded.as_bytes())?;
    ensure_private_file(path)?;
    Ok(secret)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("data")
    ));
    std::fs::write(&temp, bytes).with_context(|| format!("could not write {}", temp.display()))?;
    ensure_private_file(&temp)?;
    #[cfg(windows)]
    {
        let backup = path.with_extension("bak");
        if path.exists() {
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(path, &backup)
                .with_context(|| format!("could not replace {}", path.display()))?;
        }
        std::fs::rename(&temp, path)
            .with_context(|| format!("could not activate {}", path.display()))?;
        let _ = std::fs::remove_file(backup);
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(&temp, path)
            .with_context(|| format!("could not activate {}", path.display()))?;
    }
    Ok(())
}

fn truncate_metadata(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn hex(bytes: &[u8]) -> String {
    const CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(CHARS[(byte >> 4) as usize] as char);
        output.push(CHARS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn generate_totp_secret() -> String {
    let mut bytes = [0_u8; 20];
    OsRng.fill_bytes(&mut bytes);
    BASE32_NOPAD.encode(&bytes)
}

fn verify_totp(secret: &str, code: &str, now: i64) -> bool {
    let code = code.trim();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let Ok(code) = code.parse::<u32>() else {
        return false;
    };
    for offset in [-1_i64, 0, 1] {
        if totp_code(secret, now + offset * 30) == Some(code) {
            return true;
        }
    }
    false
}

fn totp_code(secret: &str, timestamp: i64) -> Option<u32> {
    let key = BASE32_NOPAD.decode(secret.as_bytes()).ok()?;
    let counter = (timestamp.max(0) / 30) as u64;
    let mut mac = HmacSha1::new_from_slice(&key).ok()?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let value = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | digest[offset + 3] as u32;
    Some(value % 1_000_000)
}

fn qr_svg(qr: &QrCode, border: i32) -> String {
    let size = qr.size();
    let dimension = size + border.saturating_mul(2);
    let mut path = String::new();
    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                path.push_str(&format!(
                    "M{} {}h1v1h-1z",
                    x + border,
                    y + border
                ));
            }
        }
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {dimension} {dimension}" shape-rendering="crispEdges"><rect width="{dimension}" height="{dimension}" fill="#fff"/><path d="{path}" fill="#000"/></svg>"##
    )
}

fn totp_url(username: &str, secret: &str) -> String {
    let label = percent_encode(&format!("NordTorrent:{username}"));
    let issuer = percent_encode("NordTorrent");
    format!(
        "otpauth://totp/{label}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits=6&period=30"
    )
}

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use tempfile::TempDir;

    fn service() -> (TempDir, Arc<AuthService>) {
        let dir = TempDir::new().unwrap();
        let auth = AuthService::load(
            dir.path().join("users.json"),
            &dir.path().join("secret"),
            Duration::from_secs(3600),
            false,
        )
        .unwrap();
        (dir, auth)
    }

    #[tokio::test]
    async fn setup_login_and_token_roundtrip() {
        let (_dir, auth) = service();
        assert!(auth.setup_required());
        auth.bootstrap_admin("jonathan", "correct horse battery staple")
            .await
            .unwrap();
        assert!(!auth.setup_required());
        let (user, token) = auth
            .authenticate("jonathan", "correct horse battery staple", "test")
            .await
            .unwrap();
        assert_eq!(user.role, Role::Admin);
        let identity = auth.validate_token(&token).unwrap();
        assert_eq!(identity.username, "jonathan");
    }

    #[tokio::test]
    async fn rejects_tampered_token() {
        let (_dir, auth) = service();
        auth.bootstrap_admin("jonathan", "correct horse battery staple")
            .await
            .unwrap();
        let (_, token) = auth
            .authenticate("jonathan", "correct horse battery staple", "test")
            .await
            .unwrap();
        let mut tampered = token.clone();
        tampered.push('x');
        assert!(auth.validate_token(&tampered).is_err());
    }

    #[tokio::test]
    async fn password_change_invalidates_old_token() {
        let (_dir, auth) = service();
        auth.bootstrap_admin("jonathan", "correct horse battery staple")
            .await
            .unwrap();
        let (_, token) = auth
            .authenticate("jonathan", "correct horse battery staple", "test")
            .await
            .unwrap();
        auth.change_password(
            "jonathan",
            "correct horse battery staple",
            "a new and better passphrase",
        )
        .await
        .unwrap();
        assert!(auth.validate_token(&token).is_err());
    }

    #[test]
    fn totp_matches_rfc_vector() {
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(totp_code(secret, 59), Some(287082));
    }

    #[tokio::test]
    async fn totp_is_required_remotely_but_not_locally() {
        let (_dir, auth) = service();
        auth.bootstrap_admin("jonathan", "correct horse battery staple")
            .await
            .unwrap();
        let setup = auth.setup_totp("jonathan").unwrap();
        let code = format!(
            "{:06}",
            totp_code(&setup.secret, Utc::now().timestamp()).unwrap()
        );
        auth.enable_totp("jonathan", &code).unwrap();

        let error = auth
            .authenticate_with_totp_context(
                "jonathan",
                "correct horse battery staple",
                "remote",
                "203.0.113.10",
                "Test",
                None,
                false,
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, "totp_required");

        auth.authenticate_with_totp_context(
            "jonathan",
            "correct horse battery staple",
            "remote",
            "203.0.113.10",
            "Test",
            Some(&code),
            false,
        )
        .await
        .unwrap();

        // Local clients are explicitly exempt from the TOTP challenge.
        auth.authenticate_with_totp_context(
            "jonathan",
            "correct horse battery staple",
            "local",
            "192.168.1.10",
            "Test",
            None,
            true,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn lists_and_revokes_sessions() {
        let (_dir, auth) = service();
        auth.bootstrap_admin("jonathan", "correct horse battery staple")
            .await
            .unwrap();
        let (_, token) = auth
            .authenticate_with_context(
                "jonathan",
                "correct horse battery staple",
                "test",
                "203.0.113.10",
                "Test Browser",
            )
            .await
            .unwrap();
        let identity = auth.validate_token(&token).unwrap();
        let sessions = auth.list_sessions(&identity.session_id);
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].current);
        assert_eq!(sessions[0].ip, "203.0.113.10");
        auth.revoke_session(&sessions[0].id).unwrap();
        assert!(auth.validate_token(&token).is_err());
        assert!(auth.list_sessions("").is_empty());
    }

    #[tokio::test]
    async fn login_rate_limit_engages() {
        let (_dir, auth) = service();
        auth.bootstrap_admin("jonathan", "correct horse battery staple")
            .await
            .unwrap();
        for _ in 0..MAX_LOGIN_FAILURES {
            assert!(auth.authenticate("jonathan", "wrong", "ip").await.is_err());
        }
        let err = auth
            .authenticate("jonathan", "correct horse battery staple", "ip")
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::TOO_MANY_REQUESTS);
    }
}
