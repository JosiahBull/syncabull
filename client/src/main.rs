use std::path::PathBuf;

use clap::{Parser, Subcommand};



struct Config {
    /// the location to store downloaded media
    store_path: PathBuf,
    /// whether we have been authenticated
    authenticated: bool,
    /// our id as provided by the remote server
    local_id: Option<String>,
    /// the passcode provided by the remote server
    local_passcode: Option<String>,
    /// the address of the remote server
    webserver_address_full: String,
}

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {

}




#[tokio::main]
async fn main() {
    //Spawn shutdown handler
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        println!("Ctrl-C pressed");
        //TODO
    });

    //Spawn webserver
    tokio::task::spawn(async move {

    });

    //Spawn media downloader

}
