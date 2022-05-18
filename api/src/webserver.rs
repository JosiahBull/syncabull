use std::{
    collections::BTreeMap,
    convert::Infallible,
    net::Ipv4Addr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use handlebars::Handlebars;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope,
    TokenResponse, TokenUrl,
};
use reqwest::StatusCode;
use warp::{reject::Reject, Filter, Rejection, Reply};

use crate::{
    auth::{Credentials, Token},
    json_templates::QueryData,
    photoscanner::PhotoScanner,
    AppState, GoogleAuth, UserData,
};

#[derive(Debug)]
pub struct CustomError(String, StatusCode);

impl CustomError {
    pub fn new(msg: String, status: StatusCode) -> CustomError {
        CustomError(msg, status)
    }
}

impl Reject for CustomError {}

pub async fn handle_custom_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(CustomError(msg, status)) = err.find::<CustomError>() {
        eprintln!("Rejecting a request with: {}", msg.clone());
        Ok(warp::reply::with_status(msg.clone(), *status))
    } else {
        Err(err)
    }
}

#[derive(Debug, Default)]
pub struct WebServerBuilder {
    google_client_id: Option<String>,
    google_client_secret: Option<String>,
    auth_url: Option<String>,
    token_url: Option<String>,
    domain: Option<String>,
    state: Option<Arc<AppState>>,
    handlebars: Option<Arc<Handlebars<'static>>>,
    scanner: Option<Arc<PhotoScanner>>,
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

    pub fn domain<T: Into<String>>(self, domain: T) -> Self {
        WebServerBuilder {
            domain: Some(domain.into()),
            ..self
        }
    }

    pub fn state<T: Into<Arc<AppState>>>(self, state: T) -> Self {
        WebServerBuilder {
            state: Some(state.into()),
            ..self
        }
    }

    pub fn handlebars<T: Into<Handlebars<'static>>>(self, handlebars: T) -> Self {
        let mut handlebars: Handlebars<'static> = handlebars.into();

        handlebars.set_strict_mode(true);

        WebServerBuilder {
            handlebars: Some(Arc::new(handlebars)),
            ..self
        }
    }

    pub fn scanner<T: Into<PhotoScanner>>(self, scanner: T) -> Self {
        WebServerBuilder {
            scanner: Some(Arc::new(scanner.into())),
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
            RedirectUrl::new(format!(
                "{}/api/1/callback",
                self.domain.as_ref().expect("redirect url set")
            ))
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
            .add_scope(Scope::new(String::from(
                "https://www.googleapis.com/auth/userinfo.email",
            )))
            .set_pkce_challenge(pkce_code_challenge)
            .add_extra_param("prompt", "consent")
            .add_extra_param("access_type", "offline")
            .url();

        WebServer {
            client,
            pkce_code_verifier,
            csrf_state,
            auth_url: authorize_url.to_string(),
            domain: self.domain.expect("domain set"),
            state: self.state.expect("state set"),
            handlebars: self.handlebars.expect("handlebars set"),
            scanner: self.scanner.expect("scanner set"),
        }
    }
}

pub struct WebServer {
    pub client: BasicClient,
    pub domain: String,
    pub pkce_code_verifier: PkceCodeVerifier,
    pub csrf_state: CsrfToken,
    pub auth_url: String,
    pub state: Arc<AppState>,
    pub handlebars: Arc<Handlebars<'static>>,
    pub scanner: Arc<PhotoScanner>,
}

fn with<T: Send + Sync>(
    data: Arc<T>,
) -> impl Filter<Extract = (Arc<T>,), Error = Infallible> + Clone {
    warp::any().map(move || data.clone())
}

impl WebServer {
    pub fn builder() -> WebServerBuilder {
        WebServerBuilder::default()
    }

    pub async fn register(webserver: Arc<WebServer>) -> Result<impl Reply, Infallible> {
        let mut auth: Credentials;
        let mut insecure: String;
        loop {
            (auth, insecure) = Credentials::new();
            if !webserver.state.users.read().await.contains_key(&auth.id) {
                break;
            }
        }

        webserver.state.users.write().await.insert(
            auth.id.clone(),
            UserData {
                hashed_passcode: auth.passcode,
                tokens: Vec::new(),
                google_auth: None,
            },
        );

        auth.passcode = insecure;
        Ok(warp::reply::with_status(
            warp::reply::json(&auth),
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn login(
        webserver: Arc<WebServer>,
        creds: Credentials,
    ) -> Result<impl Reply, Rejection> {
        let hashed_passcode = match webserver.state.users.read().await.get(&creds.id) {
            Some(s) => s.hashed_passcode.clone(), //clone requires alloc, but it allows us to drop the rwlock
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid login"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        if !Credentials::verify_passcode(&creds.passcode, &hashed_passcode) {
            return Err(warp::reject::custom(CustomError::new(
                String::from("invalid login"),
                StatusCode::UNAUTHORIZED,
            )));
        }

        let token = Token::generate_token(&creds.id);

        let reply = warp::reply::with_status(warp::reply::json(&token), warp::http::StatusCode::OK);
        webserver
            .state
            .tokens
            .write()
            .await
            .insert(token.token.clone(), token);

        Ok(reply)
    }

    pub async fn download(server: Arc<WebServer>, token: String) -> Result<impl Reply, Rejection> {
        let user_id = match server.state.tokens.read().await.get(&token) {
            Some(t) => t.id.clone(),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid token"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        let google_token = match server.state.users.read().await.get(&user_id) {
            Some(u) => u.google_auth.clone(),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid user"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        let google_token = match google_token {
            Some(t) => t,
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("not google authorised"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        let scanner = server.scanner.clone();

        let res = match scanner.scan(&google_token, 50, None).await {
            Ok(r) => r,
            Err(e) => {
                return Err(warp::reject::custom(CustomError::new(
                    format!("{}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )))
            }
        };

        let reply = warp::reply::with_status(warp::reply::json(&res), warp::http::StatusCode::OK);

        Ok(reply)
    }

    pub async fn get_auth_url(
        server: Arc<WebServer>,
        token: String,
    ) -> Result<impl Reply, Rejection> {
        let user_id = match server.state.tokens.read().await.get(&token) {
            Some(s) => s.id.clone(),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid token"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        let token = Token::generate_token(&user_id);

        let reply = format!("{}/api/1/auth/{}", server.domain, token.token);
        server
            .state
            .auth_keys
            .write()
            .await
            .insert(token.token.clone(), token);

        Ok(reply)
    }

    pub async fn begin_auth(
        auth_cookie: String,
        server: Arc<WebServer>,
    ) -> Result<impl Reply, Rejection> {
        //validate auth_cookie still exists
        if !server
            .state
            .auth_keys
            .read()
            .await
            .contains_key(&auth_cookie)
        {
            return Err(warp::reject::custom(CustomError::new(
                String::from("invalid url"),
                StatusCode::NOT_FOUND,
            )));
        }

        let callback_url = String::from("/api/1/callback");

        let mut data = BTreeMap::new();

        data.insert("redirect_url", &server.auth_url);
        data.insert("valid_path", &callback_url);
        data.insert("token", &auth_cookie);

        let body = server.handlebars.render("cookie", &data).unwrap();

        Ok(warp::reply::html(body))
    }

    pub async fn verify(server: Arc<WebServer>, data: QueryData) -> Result<impl Reply, Rejection> {
        let code = AuthorizationCode::new(data.code);
        // Exchange the code with a token.
        let token_server = server.clone();
        let token_response = tokio::task::spawn_blocking(move || {
            token_server
                .client
                .exchange_code(code)
                .set_pkce_verifier(PkceCodeVerifier::new(
                    token_server.pkce_code_verifier.secret().to_string(),
                ))
                .request(http_client)
        })
        .await
        .unwrap()
        .unwrap();

        let google_token = GoogleAuth {
            token: token_response.access_token().secret().to_string(),
            token_expiry_sec_epoch: SystemTime::now()
                .checked_add(Duration::from_secs(
                    token_response.expires_in().unwrap().as_secs() - 10, //lose 10 seconds, just in case
                ))
                .unwrap(),
            refresh_token: token_response.refresh_token().unwrap().secret().to_string(),
        };

        // we can't know which client this data is associated with, so we need the user to do that for us
        // here we are going to generate a token representing this login, and then the associated client
        // needs to make a post request to claim their login officially and associate it with the correct
        // account.
        // At some point we should make this process cryptographic, e.g. the user must sign the token with
        // the key they were provided with earlier verifiably.
        // But for now this is adequate.

        //blank id provided, the user should fill this with their token when returning it
        let token = Token::generate_token(&String::with_capacity(0));

        let mut data = BTreeMap::new();
        data.insert("token", serde_json::to_string(&token).unwrap());
        data.insert(
            "post_url",
            format!("{}/api/1/token_completion", server.domain),
        );

        let body = server.handlebars.render("success", &data).unwrap();

        server
            .state
            .unclaimed_auth_tokens
            .write()
            .await
            .insert(token.token, google_token);

        Ok(warp::reply::html(body))
    }

    pub async fn token_completion(
        server: Arc<WebServer>,
        data: Token,
    ) -> Result<impl Reply, Rejection> {
        // This endpoint is used to validate and finalise a login

        //check that the provided cookie is valid
        let unauth_user = data.id;
        let user = match server.state.auth_keys.write().await.remove(&unauth_user) {
            Some(s) => s,
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid token"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        //validate there is an unclaimed login
        let unclaimed_token = data.token;
        let unclaimed_login = match server
            .state
            .unclaimed_auth_tokens
            .write()
            .await
            .remove(&unclaimed_token)
        {
            Some(s) => s,
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid token"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        //login this user
        match server.state.users.write().await.get_mut(&user.id) {
            Some(s) => s.google_auth = Some(unclaimed_login),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid login"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        }

        Ok(warp::reply::with_status(
            warp::reply(),
            StatusCode::NO_CONTENT,
        ))
    }

    pub async fn login_check(
        _: Arc<WebServer>,
        _: String,
    ) -> Result<impl Reply, Infallible> {
        Ok(String::from("not implemented"))
    }

    pub async fn delete_data(
        webserver: Arc<WebServer>,
        token: String,
    ) -> Result<impl Reply, Rejection> {
        let user_id = match webserver.state.tokens.read().await.get(&token) {
            Some(t) => t.id.clone(),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid login"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        let user = match webserver.state.users.write().await.remove(&user_id) {
            Some(u) => u,
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid user state, token not removed"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        // remove all tokens for this user
        if !user.tokens.is_empty() {
            let mut writer = webserver.state.tokens.write().await;
            for token in user.tokens {
                writer.remove(&token);
            }
        }

        Ok(warp::reply::with_status("", StatusCode::NO_CONTENT))
    }

    pub async fn run(self) {
        let webserver = Arc::new(self);

        // register this agent with the api
        let register = warp::get()
            .and(warp::path("register"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and_then(WebServer::register)
            .recover(handle_custom_error);

        // log this agent into the api
        let login = warp::post()
            .and(warp::path("login"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::body::json::<Credentials>())
            .and_then(WebServer::login)
            .recover(handle_custom_error);

        // check for new images to download
        let download = warp::get()
            .and(warp::path("download"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::header::header::<String>("authorisation"))
            .and_then(WebServer::download)
            .recover(handle_custom_error);

        // this endpoint is used to generate a login url for the google auth process
        // the user will be given this url to visit to begin the login process
        let get_auth_url = warp::get()
            .and(warp::path("auth_url"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::header::header::<String>("authorisation"))
            .and_then(WebServer::get_auth_url)
            .recover(handle_custom_error);

        // this endpoint is the beginning of the google login process
        // the url generated by the above links to this endpoint
        // this endpoint should log a cookie into localstorage, then redirect to google's api
        // to complete the login process
        let auth = warp::get()
            .and(warp::path("auth"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and_then(WebServer::begin_auth)
            .recover(handle_custom_error);

        // after the user has completed the google api login process, it will redirect them here
        // this api should validate the provided OTC from the google api
        // and if successful, serve the user an page which will validate the token saved earlier
        // this is important so we can associate a user with a token
        let auth_callback = warp::get()
            .and(warp::path("callback"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::query::<QueryData>())
            .and_then(WebServer::verify)
            .recover(handle_custom_error);

        // when a user has autho
        let auth_token_completion = warp::post()
            .and(warp::path("token_completion"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::body::json::<Token>())
            .and_then(WebServer::token_completion)
            .recover(handle_custom_error);

        // long poll for user login succeeding
        let login_check = warp::get()
            .and(warp::path("is_logged_in"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::header::header::<String>("authorisation"))
            .and_then(WebServer::login_check)
            .recover(handle_custom_error);

        // delete all data associated with this user
        let delete_data = warp::delete()
            .and(warp::path("delete"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::header::header::<String>("authorisation"))
            .and_then(WebServer::delete_data)
            .recover(handle_custom_error);

        // General catch-all endpoint if a failure occurs
        let catcher = warp::any()
            .and(warp::path::full())
            .map(|path| format!("Path {:?} not found", path));

        //TODO: refactor this.
        // every route needs webserver, so lets do that here
        // routes that require authorisation should be done here - avoids accidentally not authorising a route
        // routes shoudl be authorised and rejected here *not* inside of the functions
        let routes = warp::any()
            .and(warp::path("api"))
            .and(warp::path("1"))
            .and(
                register
                    .or(login)
                    .or(download)
                    .or(get_auth_url)
                    .or(auth)
                    .or(auth_callback)
                    .or(auth_token_completion)
                    .or(login_check)
                    .or(delete_data),
            )
            .or(catcher);

        warp::serve(routes)
            .run((
                std::env::var("HOST")
                    .expect("HOST to be set")
                    .parse::<Ipv4Addr>()
                    .expect("valid port"),
                std::env::var("PORT")
                    .expect("PORT to be set")
                    .parse()
                    .expect("valid port"),
            ))
            .await;
    }
}
