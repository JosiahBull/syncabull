mod json_templates;
mod photoscanner;
mod webserver;

use std::{sync::Arc, collections::HashMap};
use photoscanner::{PhotoScanner, UserData};
use tokio::sync::RwLock;
use webserver::WebServer;


pub type Users = Arc<RwLock<HashMap<String, UserData>>>;


#[tokio::main]
async fn main() {
    let users: Users = Default::default();

    //TODO: database integration

    tokio::task::spawn(async {
        WebServer::init().await.run().await;
    });


    tokio::task::spawn(async {
        PhotoScanner::init(users).await.run().await;
    });
}
