use std::sync::Arc;
use std::{convert::Infallible, env};

use oauth2::reqwest::http_client;
use oauth2::{
    basic::BasicClient, url::Url, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenUrl,
};
use warp::{http::Response, Filter, Reply};

use crate::json_templates::QueryData;

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
    async fn handle_auth_request(
        server: Arc<WebServer>,
        data: QueryData,
    ) -> Result<impl Reply, Infallible> {
        let code = AuthorizationCode::new(data.code);
        let state = CsrfToken::new(data.state);

        println!("Google returned the following code:\n{}\n", code.secret());
        println!(
            "Google returned the following state:\n{} (expected `{}`)\n",
            state.secret(),
            server.csrf_state.secret()
        );

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

        println!(
            "Google returned the following token:\n{:?}\n",
            token_response
        );

        Ok(Response::builder().body("Go back to your terminal :)"))
    }

    pub async fn init() -> Self {
        //Load enviro variables
        //TODO: not hardcoded
        let google_client_id = ClientId::new(
            env::var("GOOGLE_CLIENT_ID")
                .expect("Missing the GOOGLE_CLIENT_ID environment variable."),
        );
        let google_client_secret = ClientSecret::new(
            env::var("GOOGLE_CLIENT_SECRET")
                .expect("Missing the GOOGLE_CLIENT_SECRET environment variable."),
        );
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
            .expect("Invalid token endpoint URL");

        // Google auth client setup
        let client = BasicClient::new(
            google_client_id,
            Some(google_client_secret),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(
            RedirectUrl::new("http://localhost:8080".to_string()).expect("Invalid redirect URL"),
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
            .url();

        WebServer {
            client,
            pkce_code_verifier,
            csrf_state,
            url: authorize_url,
        }
    }

    pub async fn run(self) {
        let server = Arc::new(self);

        // Load the url used for authentication
        let auth_url = warp::get()
        .and(warp::path("url"))
        .and(warp::path::end())
        .and(with_extra(server.clone()))
        .map(move |server: Arc<WebServer>| {
            Ok(Response::builder()
                .header("Content-Type", "text/html")
                .body(format!(
                    "{}",
                    server.url
                ))
                .unwrap())
        });

        // Authentication request outcome
        let auth = warp::get()
            .and(warp::path("auth"))
            .and(warp::path::end())
            .and(with_extra(server.clone()))
            .and(warp::query::<QueryData>())
            .and_then(WebServer::handle_auth_request);

        // Server Authentication

        let routes = auth_url.or(auth);

        warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
    }
}
