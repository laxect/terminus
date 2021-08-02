pub(crate) struct Config {
    pub endpoint: String,
    pub username: String,
    pub pass: String,
}

impl Config {
    pub(crate) fn from_file() -> Self {
        Config {
            endpoint: "[::1]:1120".to_owned(),
            username: "名無し".to_owned(),
            pass: "CzwnmURw8".to_owned(),
        }
    }

    pub(crate) fn save_to_file() {}
}
