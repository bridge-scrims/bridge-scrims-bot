use serenity::{
    prelude::Context,
    async_trait
};
use serde_json::{value::Value, from_reader};
use crate::commands::Command;
use std::sync::Arc;
use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Field {
    name: String,
    value: String
}

#[derive(Debug, Deserialize, Serialize)]
struct Embed {
    title: String,
    description: String,
    fields: Vec<Field>,
    footer: String,
    color: String
}

#[derive(Debug, Deserialize, Serialize)]
struct Message {
    embed: Option<Embed>,
    content: Option<String>
}

pub struct Prefab {
    inner: Arc<Inner>
}

struct Inner {
}

#[async_trait]
impl Command for Prefab {
    fn new() -> Box<Self>
    where
        Self: Sized,
    {
        let data_str = fs::read_to_string("prefabs.json");
        println!("{:?}", data_str);
        Box::new(Prefab {
            inner: Arc::new(Inner {
                
            }),
        })
    }
}