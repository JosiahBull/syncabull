use crate::Users;

pub struct UserData {
    UserID: u32,
    google_token: Option<String>,
    api_limit_current: usize,
    current_page_id: Option<String>,
    initial_scan_completed: bool,
    last_check_epoch: u64,
    newly_added: bool,
}


pub struct PhotoScanner {

}


impl PhotoScanner {
    pub async fn init(users: Users) -> Self {
        todo!()
    }

    pub async fn run(self) {

    }
}
