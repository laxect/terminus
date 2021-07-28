use base64::encode;
use blake3::hash;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// u128 is 8 * 16
fn get_id(tail: u128) -> u128 {
    let unix_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Hello John titor!")
        .as_secs() as u128;
    (unix_timestamp << 64) + tail
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Pass {
    Pass(String),
    Mask(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Author {
    name: String,
    pass: Pass,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    id: Vec<u128>,
    title: String,
    author: Author,
    content: String,
}

impl Author {
    pub fn new(name: String, pass: String) -> Self {
        Self {
            name,
            pass: Pass::Pass(pass),
        }
    }

    pub fn mask(self) -> Self {
        match self.pass {
            Pass::Mask(_) => self,
            Pass::Pass(pass) => {
                let mut input = self.name.clone();
                input.push_str(&pass);
                Self {
                    name: self.name,
                    pass: Pass::Mask(encode(hash(input.as_bytes()).as_bytes())),
                }
            }
        }
    }

    pub fn unlock(&self, name: String, pass: String) -> bool {
        if self.name != name {
            return false;
        }
        match self.pass {
            Pass::Mask(ref inner_pass) => {
                let mut input = name;
                input.push_str(&pass);
                inner_pass == &encode(hash(input.as_bytes()).as_bytes())
            }
            Pass::Pass(ref inner_pass) => inner_pass == &pass,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Author;

    #[test]
    fn mask_unlock() {
        let name = String::from("donadona");
        let pass = String::from("xmicjsuUHXahuxaHU");
        let author = Author::new(name.clone(), pass.clone());
        let author = author.mask();
        assert!(author.unlock(name, pass));
    }
}
