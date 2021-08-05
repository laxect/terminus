use crate::ui::panel::Input;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};
use terminus_types::{Author, Pass};

pub const APPLICATION: &str = "kanban";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Config {
    pub endpoint: String,
    pub username: String,
    pub password: String,
}

fn default_password() -> String {
    let mut rng = rand::thread_rng();
    let mut number = [0u8; 16];
    rng.fill(&mut number);
    blake3::hash(&number).to_hex().to_lowercase()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            endpoint: "[::1]:1120".to_owned(),
            username: "名無し".to_owned(),
            password: default_password(),
        }
    }
}

impl Config {
    pub(crate) fn from_file() -> anyhow::Result<Self> {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join(APPLICATION)
            .join("config.toml");
        let mut config = String::new();
        File::open(path)?.read_to_string(&mut config)?;
        Ok(toml::from_str(&config)?)
    }

    pub(crate) fn save_to_file(&self) -> anyhow::Result<()> {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join(APPLICATION);
        create_dir_all(&path).ok();
        let path = path.join("config.toml");
        let mut config_file = OpenOptions::new().create_new(true).write(true).open(path)?;
        let config_str = toml::to_string_pretty(&self)?;
        config_file.write_all(config_str.as_bytes())?;
        Ok(())
    }

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
