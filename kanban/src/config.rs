use terminus_types::{Author, Pass};

use crate::ui::panel::Input;

pub(crate) struct Config {
    pub endpoint: String,
    pub username: String,
    pub password: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            endpoint: "[::1]:1120".to_owned(),
            username: "名無し".to_owned(),
            password: "CzwnmURw8".to_owned(),
        }
    }
}

impl Config {
    pub(crate) fn from_file() -> anyhow::Result<Self> {
        Ok(Config {
            endpoint: "[::1]:1120".to_owned(),
            username: "名無し".to_owned(),
            password: "CzwnmURw8".to_owned(),
        })
    }

    pub(crate) fn save_to_file(&self) {}

    pub(crate) fn gen_inputs(&self) -> Vec<Input> {
        vec![
            Input::new("endpoint", &self.endpoint, false),
            Input::new("username", &self.username, false),
            Input::new("password", &self.password, false),
        ]
    }

    pub(crate) fn set_val_from_inputs(&mut self, inputs: &[Input]) {
        for Input { label, input, .. } in inputs {
            match label.as_str() {
                "endpoint" => {
                    self.endpoint = input.to_owned();
                }
                "username" => {
                    self.username = input.to_owned();
                }
                "password" => {
                    self.password = input.to_owned();
                }
                _ => unreachable!(),
            }
        }
    }

    pub(crate) fn gen_author(&self) -> Author {
        Author {
            name: self.username.clone(),
            pass: Pass::Pass(self.password.clone()),
        }
    }
}
