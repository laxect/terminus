use action::Action;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod action;
mod error;

pub use error::{Error, Result};

// u128 is 8 * 16
fn get_id(tail: u128) -> u128 {
    let unix_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Hello John titor!")
        .as_secs() as u128;
    (unix_timestamp << 64) + tail
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Pass {
    Pass(String),
    Mask(String),
}

impl Pass {
    pub fn get_pass(&self) -> &str {
        match self {
            Pass::Mask(pass) => pass,
            Pass::Pass(pass) => pass,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Author {
    pub name: String,
    pub pass: Pass,
}

impl Author {
    pub fn is_masked(&self) -> bool {
        matches!(self.pass, Pass::Mask(_))
    }
}

pub type NodeId = Vec<u128>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub title: String,
    pub author: Author,
    pub content: String,
    pub publish_time: DateTime<Utc>,
    pub edited: bool,
}

fn mask_name_pass(name: &str, pass: &str) -> String {
    let mut input = name.to_owned();
    input.push_str(pass);
    base64::encode(blake3::hash(input.as_bytes()).as_bytes())
}

impl Author {
    pub fn new(name: String, pass: String) -> Self {
        Self {
            name,
            pass: Pass::Pass(pass),
        }
    }

    pub fn mask(&mut self) {
        match self.pass {
            Pass::Mask(_) => {}
            Pass::Pass(ref pass) => {
                let pass = mask_name_pass(&self.name, pass);
                self.pass = Pass::Mask(pass);
            }
        }
    }

    pub fn match_pass(&self, name: &str, pass: &str) -> bool {
        if self.name != name {
            return false;
        }
        match self.pass {
            Pass::Mask(ref inner_pass) => inner_pass == &mask_name_pass(name, pass),
            Pass::Pass(ref inner_pass) => inner_pass == pass,
        }
    }
}

impl Node {
    pub fn new(parent_id: &[u128], title: String, author: Author, content: String, tail: u64) -> Self {
        let mut id = parent_id.to_owned();
        id.push(get_id(tail as u128));
        Self {
            id,
            title,
            author,
            content,
            publish_time: Utc::now(),
            edited: false,
        }
    }

    pub fn post(mut self) -> Action {
        if !self.author.is_masked() {
            self.author.mask();
        }
        Action::Post(self)
    }

    pub fn update(mut self) -> Result<Action> {
        if self.author.is_masked() {
            return Err(Error::NeedUnMaskPass);
        }
        self.edited = true;
        Ok(Action::Update(self))
    }

    /// delete need to send pass and id, all other field is not needed.
    /// but for clean api, all them has same pattern. may change in the future.
    pub fn delete(self) -> Result<Action> {
        if self.author.is_masked() {
            return Err(Error::NeedUnMaskPass);
        }
        Ok(Action::Delete(self))
    }

    /// the last part of id.
    pub fn last_id(&self) -> Result<u128> {
        // should always have one
        self.id.last().ok_or(Error::IdInvalid).map(|x| *x)
    }
}

#[cfg(test)]
mod tests {
    use super::Author;

    #[test]
    fn mask_unlock() {
        let name = String::from("donadona");
        let pass = String::from("xmicjsuUHXahuxaHU");
        let mut author = Author::new(name.clone(), pass.clone());
        author.mask();
        assert!(author.match_pass(&name, &pass));
    }
}