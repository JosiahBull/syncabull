use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use oauth2::reqwest::http_client;
use oauth2::{
    basic::BasicClient, url::Url, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenResponse,
    TokenUrl,
};
use warp::{http::Response, Filter, Reply};

use crate::json_templates::QueryData;
use crate::{AppState, AuthToken, GoogleAuth, UserState};

#[derive(Debug, Default)]
pub struct WebServerBuilder {
    google_client_id: Option<String>,
    google_client_secret: Option<String>,
    auth_url: Option<String>,
    token_url: Option<String>,
    redirect_url: Option<String>,
    state: Option<Arc<AppState>>,
}

impl WebServerBuilder {
    pub fn google_client_id<T: Into<String>>(self, google_client_id: T) -> Self {
        WebServerBuilder {
            google_client_id: Some(google_client_id.into()),
            ..self
        }
    }

    pub fn google_client_secret<T: Into<String>>(self, google_client_secret: T) -> Self {
        WebServerBuilder {
            google_client_secret: Some(google_client_secret.into()),
            ..self
        }
    }

    pub fn auth_url<T: Into<String>>(self, auth_url: T) -> Self {
        WebServerBuilder {
            auth_url: Some(auth_url.into()),
            ..self
        }
    }

    pub fn token_url<T: Into<String>>(self, token_url: T) -> Self {
        WebServerBuilder {
            token_url: Some(token_url.into()),
            ..self
        }
    }

    pub fn redirect_url<T: Into<String>>(self, redirect_url: T) -> Self {
        WebServerBuilder {
            redirect_url: Some(redirect_url.into()),
            ..self
        }
    }

    pub fn state<T: Into<Arc<AppState>>>(self, state: T) -> Self {
        WebServerBuilder {
            state: Some(state.into()),
            ..self
        }
    }

    pub fn build(self) -> WebServer {
        let google_client_id = ClientId::new(self.google_client_id.expect("google_client_id set"));
        let google_client_secret =
            ClientSecret::new(self.google_client_secret.expect("google_client_secret set"));
        let auth_url = AuthUrl::new(self.auth_url.expect("auth_url set")).expect("auth_url parse");
        let token_url =
            TokenUrl::new(self.token_url.expect("token_url set")).expect("token_url parse");

        // Google auth client setup
        let client = BasicClient::new(
            google_client_id,
            Some(google_client_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(
            RedirectUrl::new(self.redirect_url.expect("redirect url set"))
                .expect("Invalid redirect URL"),
        )
        .set_revocation_uri(
            RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())
                .expect("Invalid revocation endpoint URL"),
        );

        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(String::from(
                "https://www.googleapis.com/auth/photoslibrary.readonly",
            )))
            .add_scope(Scope::new(String::from(
                "https://www.googleapis.com/auth/plus.me",
            )))
            .set_pkce_challenge(pkce_code_challenge)
            .add_extra_param("prompt", "consent")
            .add_extra_param("access_type", "offline")
            .url();

        WebServer {
            client,
            pkce_code_verifier,
            csrf_state,
            url: authorize_url,
        }
    }
}

pub struct WebServer {
    pub client: BasicClient,
    pub pkce_code_verifier: PkceCodeVerifier,
    pub csrf_state: CsrfToken,
    pub url: Url,
}

fn with_extra(
    arcer: Arc<WebServer>,
) -> impl Filter<Extract = (Arc<WebServer>,), Error = Infallible> + Clone {
    warp::any().map(move || arcer.clone())
}

impl WebServer {
    pub fn builder() -> WebServerBuilder {
        WebServerBuilder::default()
    }

    async fn handle_auth_request(
        server: Arc<WebServer>,
        data: QueryData,
    ) -> Result<impl Reply, Infallible> {

        let code = AuthorizationCode::new(data.code);
        // Exchange the code with a token.
        let token_response = tokio::task::spawn_blocking(move || {
            server
                .client
                .exchange_code(code)
                .set_pkce_verifier(PkceCodeVerifier::new(
                    server.pkce_code_verifier.secret().to_string(),
                ))
                .request(http_client)
        })
        .await
        .unwrap()
        .unwrap();

        // TODO: load user data
        // TODO: we should probably encrypt these tokens using something like sodium
        let user: UserState = UserState {
            user_id: todo!(),
            email: String::from(""),
            auth_token: todo!(),
            google_token: GoogleAuth {
                token: token_response.access_token().secret().to_string(),
                token_expiry_sec_epoch: SystemTime::now()
                    .checked_add(Duration::from_secs(
                        token_response.expires_in().unwrap().as_secs(),
                    ))
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .expect("time went backwards?")
                    .as_secs()
                    - 10, //lose 10 seconds, just in case
                refresh_token: token_response.refresh_token().unwrap().secret().to_string(),
            },
            next_token: None,
            initial_scan_completed: false,
            last_checked: u64::MAX,
            photos_scanned: 0,
        };

        Ok(Response::builder().body("Authorisation Completed"))
    }

    pub async fn handle_download_request(
        server: Arc<WebServer>,
        authorisation: Option<AuthToken>,
    ) -> Result<impl Reply, Infallible> {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        Ok(Response::builder().body("Go back to your terminal :)"))
    }

    pub async fn run(self) {
        let server = Arc::new(self);

        // Load the url used for authentication
        let auth_url = warp::get()
            .and(warp::path("login"))
            .and(warp::path::end())
            .and(with_extra(server.clone()))
            .map(move |server: Arc<WebServer>| {
                Ok(Response::builder()
                    .header("Content-Type", "text/html")
                    .body(format!("{}", server.url))
                    .unwrap())
            });

        // Authentication request outcome
        let auth = warp::get()
            .and(warp::path("auth"))
            .and(warp::path::end())
            .and(with_extra(server.clone()))
            .and(warp::query::<QueryData>())
            .and_then(WebServer::handle_auth_request);

        // Get a set of urls from the google api
        let download_images = warp::get()
            .and(warp::path("download"))
            .and(warp::path::end())
            .and(with_extra(server.clone()))
            .and(warp::header::optional::<AuthToken>("authorisation"))
            .and_then(WebServer::handle_download_request);

        // Server Authentication
        let routes = warp::any()
            .and(warp::path("api"))
            .and(warp::path("1"))
            .and(auth_url.or(auth).or(download_images));

        warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
    }
}
