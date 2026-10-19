use std::time::Duration;

/// Time allowed to establish a connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Time allowed for a whole request, including the upload. Generous enough for
/// a 50 MB file over a slow (~1 Mbit/s) uplink, short enough that a stalled
/// connection doesn't hang an agent forever.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The HTTP client every backend uses.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("failed to build HTTP client")
}
