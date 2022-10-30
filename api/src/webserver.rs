use std::{
    collections::BTreeMap,
    convert::Infallible,
    net::Ipv4Addr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use handlebars::Handlebars;
use oauth2::{
    basic::BasicClient,
    http::HeaderValue,
    reqwest::http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::StatusCode;
use shared_libs::json_templates::{QueryData, RequestParameters};
use tokio::{sync::RwLock, time::error::Elapsed};
use warp::{reject::Reject, Filter, Rejection, Reply};

use crate::{
    auth::{Credentials, Token},
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
    state: Option<Arc<RwLock<AppState>>>,
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

    pub fn state<T: Into<Arc<RwLock<AppState>>>>(self, state: T) -> Self {
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
    pub state: Arc<RwLock<AppState>>,
    pub handlebars: Arc<Handlebars<'static>>,
    pub scanner: Arc<PhotoScanner>,
}

fn with<T: Send + Sync>(
    data: Arc<T>,
) -> impl Filter<Extract = (Arc<T>,), Error = Infallible> + Clone {
    warp::any().map(move || data.clone())
}

pub fn with_auth(
    server: Arc<WebServer>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    warp::header::value("authorization")
        .and(with(server))
        .and_then(WebServer::login)
}

impl WebServer {
    pub fn builder() -> WebServerBuilder {
        WebServerBuilder::default()
    }

    async fn login(token: HeaderValue, webserver: Arc<WebServer>) -> Result<String, Rejection> {
        let token = token.to_str().map_err(|e| {
            CustomError::new(format!("Invalid token: {}", e), StatusCode::BAD_REQUEST)
        })?;

        if token.len() < 5 {
            return Err(warp::reject::custom(CustomError::new(
                String::from("invalid token"),
                StatusCode::UNAUTHORIZED,
            )));
        }

        let data = base64::decode(&token[6..]).map_err(|e| {
            CustomError::new(
                format!("Invalid base64 encoding: {}", e),
                StatusCode::BAD_REQUEST,
            )
        })?;

        let data = String::from_utf8(data).map_err(|e| {
            CustomError::new(
                format!("Invalid UTF-8 encoding: {}", e),
                StatusCode::BAD_REQUEST,
            )
        })?;

        if data.matches(':').count() != 1 {
            return Err(warp::reject::custom(CustomError::new(
                String::from("Invalid auth string"),
                StatusCode::BAD_REQUEST,
            )));
        }

        let mut split = data.split(':');
        let username = match split.next() {
            Some(username) => username.to_string(),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("Invalid auth string"),
                    StatusCode::BAD_REQUEST,
                )))
            }
        };

        let passcode = match split.next() {
            Some(passcode) => passcode.to_string(),
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("Invalid auth string"),
                    StatusCode::BAD_REQUEST,
                )))
            }
        };

        let hashed_passcode = match webserver.state.read().await.users.get(&username) {
            Some(s) => s.hashed_passcode.clone(), //clone requires alloc, but it allows us to drop the rwlock
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid login"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        if !Credentials::verify_passcode(&passcode, &hashed_passcode) {
            return Err(warp::reject::custom(CustomError::new(
                String::from("invalid login"),
                StatusCode::UNAUTHORIZED,
            )));
        }

        Ok(username)
    }

    pub async fn register(webserver: Arc<WebServer>) -> Result<impl Reply, Infallible> {
        let mut writer = webserver.state.write().await;

        let mut auth: Credentials;
        let mut insecure: String;
        loop {
            (auth, insecure) = Credentials::new();
            if !writer.users.contains_key(&auth.id) {
                break;
            }
        }

        writer.users.insert(
            auth.id.clone(),
            UserData {
                hashed_passcode: auth.passcode,
                tokens: Vec::new(),
                google_auth: None,
                initial_scan_complete: false,
                next_token: None,
                prev_token: None,
            },
        );

        auth.passcode = insecure;
        Ok(warp::reply::with_status(
            warp::reply::json(&auth),
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn download(
        server: Arc<WebServer>,
        settings: RequestParameters,
        user_id: String,
    ) -> Result<impl Reply, Rejection> {
        let token;
        let google_token;
        {
            let reader = server.state.read().await;
            match reader.users.get(&user_id) {
                Some(u) => {
                    token = match settings.reload {
                        true => u.prev_token.clone(),
                        false => u.next_token.clone(),
                    };
                    google_token = u.google_auth.clone();
                }
                None => {
                    return Err(warp::reject::custom(CustomError::new(
                        String::from("invalid user"),
                        StatusCode::UNAUTHORIZED,
                    )))
                }
            };
        }

        let mut google_token = match google_token {
            Some(t) => t,
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("not google authorised"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        if google_token.is_expired() {
            //refresh token
            let token_server = server.clone();
            let refresh_token = oauth2::RefreshToken::new(google_token.refresh_token.clone());
            let new_token = tokio::task::spawn_blocking(move || {
                token_server
                    .client
                    .exchange_refresh_token(&refresh_token)
                    .request(http_client)
            })
            .await
            .unwrap()
            .unwrap();

            let new_token = GoogleAuth {
                token: new_token.access_token().secret().to_string(),
                token_expiry_sec_epoch: SystemTime::now()
                    .checked_add(Duration::from_secs(
                        new_token.expires_in().unwrap().as_secs() - 10, //lose 10 seconds, just in case
                    ))
                    .unwrap(),
                refresh_token: google_token.refresh_token,
            };

            {
                let mut writer = server.state.write().await;
                writer.users.get_mut(&user_id).unwrap().google_auth = Some(new_token.clone());
                google_token = new_token;
            }
        }

        let res = match server
            .scanner
            .scan(&google_token, settings.max_count, token)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(warp::reject::custom(CustomError::new(
                    format!("{}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )))
            }
        };

        {
            let mut writer = server.state.write().await;
            let mut user = writer.users.get_mut(&user_id).unwrap();
            user.prev_token = user.next_token.clone();
            user.next_token = res.nextPageToken;

            if user.next_token.is_none() {
                user.initial_scan_complete = true;
            }
        }

        let reply = warp::reply::with_status(
            warp::reply::json(&res.mediaItems),
            warp::http::StatusCode::OK,
        );

        Ok(reply)
    }

    pub async fn get_auth_url(
        server: Arc<WebServer>,
        user_id: String,
    ) -> Result<impl Reply, Rejection> {
        let token = Token::generate_token(&user_id);

        let reply = format!("{}/api/1/auth/{}", server.domain, token.token);
        server
            .state
            .write()
            .await
            .auth_keys
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
            .read()
            .await
            .auth_keys
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
            .write()
            .await
            .unclaimed_auth_tokens
            .insert(token.token, google_token);

        Ok(warp::reply::html(body))
    }

    pub async fn token_completion(
        server: Arc<WebServer>,
        data: Token,
    ) -> Result<impl Reply, Rejection> {
        // This endpoint is used to validate and finalise a login
        let mut writer = server.state.write().await;

        //check that the provided cookie is valid
        let unauth_user = data.id;
        let user = match writer.auth_keys.remove(&unauth_user) {
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
        let unclaimed_login = match writer.unclaimed_auth_tokens.remove(&unclaimed_token) {
            Some(s) => s,
            None => {
                return Err(warp::reject::custom(CustomError::new(
                    String::from("invalid token"),
                    StatusCode::UNAUTHORIZED,
                )))
            }
        };

        //login this user
        match writer.users.get_mut(&user.id) {
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
        webserver: Arc<WebServer>,
        user_id: String,
    ) -> Result<impl Reply, Rejection> {
        //XXX move timeout to config option?

        let timeout_secs = 200;

        let result: Result<Result<(), Rejection>, Elapsed> =
            tokio::time::timeout(Duration::from_secs(timeout_secs), async move {
                loop {
                    let reader = webserver.state.read().await;
                    let user = match reader.users.get(&user_id) {
                        Some(s) => s,
                        None => {
                            return Err(warp::reject::custom(CustomError::new(
                                String::from("invalid login"),
                                StatusCode::UNAUTHORIZED,
                            )))
                        }
                    };

                    if user.google_auth.is_some() {
                        return Ok(());
                    }

                    //sleep before checking again
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            })
            .await;

        match result {
            Ok(Err(e)) => Err(e),
            Ok(_) => Ok(warp::reply::with_status(
                warp::reply(),
                StatusCode::NO_CONTENT,
            )),
            Err(_) => Err(warp::reject::custom(CustomError::new(
                String::from("invalid login"),
                StatusCode::UNAUTHORIZED,
            ))),
        }
    }

    pub async fn delete_data(
        webserver: Arc<WebServer>,
        user_id: String,
    ) -> Result<impl Reply, Rejection> {
        let mut writer = webserver.state.write().await;
        if writer.users.remove(&user_id).is_none() {
            return Err(warp::reject::custom(CustomError::new(
                String::from("invalid user"),
                StatusCode::UNAUTHORIZED,
            )));
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

        // check for new images to download
        let download = warp::get()
            .and(warp::path("download"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(warp::query::<RequestParameters>())
            .and(with_auth(webserver.clone()))
            .and_then(WebServer::download)
            .recover(handle_custom_error);

        // this endpoint is used to generate a login url for the google auth process
        // the user will be given this url to visit to begin the login process
        let get_auth_url = warp::get()
            .and(warp::path("auth_url"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(with_auth(webserver.clone()))
            .and_then(WebServer::get_auth_url)
            .recover(handle_custom_error);

        // this endpoint is the beginning of the google login process
        // the url generated by the above links to this endpoint
        // this endpoint should log a cookie into local storage, then redirect to google's api
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
            .and(with_auth(webserver.clone()))
            .and_then(WebServer::login_check)
            .recover(handle_custom_error);

        // delete all data associated with this user
        let delete_data = warp::delete()
            .and(warp::path("delete"))
            .and(warp::path::end())
            .and(with(webserver.clone()))
            .and(with_auth(webserver.clone()))
            .and_then(WebServer::delete_data)
            .recover(handle_custom_error);

        // General catch-all endpoint if a failure occurs
        let catcher = warp::any().and(warp::path::full()).map(|path| {
            warp::reply::with_status(format!("Path {:?} not found", path), StatusCode::NOT_FOUND)
        });

        //TODO: refactor this.
        // every route needs webserver, so lets do that here
        // routes that require authorisation should be done here - avoids accidentally not authorising a route
        // routes shoudl be authorised and rejected here *not* inside of the functions
        let api_1 = warp::any().and(warp::path("api")).and(warp::path("1")).and(
            register
                .or(download)
                .or(get_auth_url)
                .or(auth)
                .or(auth_callback)
                .or(auth_token_completion)
                .or(login_check)
                .or(delete_data),
        );

        let routes = warp::any().and(api_1.or(catcher));

        println!(
            "binding to : {}:{}",
            std::env::var("HOST").expect("HOST not set"),
            std::env::var("PORT").expect("PORT not set")
        );

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

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    use warp::{http::HeaderMap, Filter};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_test() {
        let h = tokio::task::spawn(async move {
            let routes = warp::any()
                .and(warp::header::headers_cloned())
                .map(|headers: HeaderMap| format!("You gave me headers: {:?}", headers));

            warp::serve(routes)
                .run((
                    "127.0.0.1".parse::<Ipv4Addr>().expect("valid port"),
                    "8000".parse().expect("valid port"),
                ))
                .await;
        });

        let client = reqwest::Client::new();
        let res = client
            .get("http://127.0.0.1:8000/hello")
            .basic_auth("username", Some("password"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        println!("got: {}", res);

        h.abort();

        panic!("incomplete");
    }
}
