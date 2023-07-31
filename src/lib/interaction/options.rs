use serde_json::{Map, Value};
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction as ACI;
use serenity::model::application::interaction::application_command::CommandDataOption as CDO;
use serenity::model::application::interaction::autocomplete::AutocompleteInteraction as AI;

/// Extracts options from an `ApplicationCommandInteraction`
/// Delegates [`serde_json::Value`] methods
pub trait InteractOpts: Sized {
    fn get_map<T>(&self, query: impl AsRef<str>, map: impl FnOnce(Value) -> T) -> Option<T>;
    fn get_array(&self, query: impl AsRef<str>) -> Option<Vec<Value>> {
        self.get_map(query, |x| {
            if let Value::Array(v) = x {
                Some(v)
            } else {
                None
            }
        })
        .flatten()
    }
    fn get_bool(&self, query: impl AsRef<str>) -> Option<bool> {
        self.get_map(query, |x| x.as_bool()).flatten()
    }

    fn get_f64(&self, query: impl AsRef<str>) -> Option<f64> {
        self.get_map(query, |x| x.as_f64()).flatten()
    }

    fn get_i64(&self, query: impl AsRef<str>) -> Option<i64> {
        self.get_map(query, |x| x.as_i64()).flatten()
    }

    fn get_null(&self, query: impl AsRef<str>) -> Option<()> {
        self.get_map(query, |x| x.as_null()).flatten()
    }

    fn get_object(&self, query: impl AsRef<str>) -> Option<Map<String, Value>> {
        self.get_map(query, |x| x.as_object().cloned()).flatten()
    }

    fn get_str(&self, query: impl AsRef<str>) -> Option<String> {
        self.get_map(query, |x| {
            if let Value::String(v) = x {
                Some(v)
            } else {
                None
            }
        })
        .flatten()
    }

    fn get_u64(&self, query: impl AsRef<str>) -> Option<u64> {
        self.get_map(query, |x| x.as_u64()).flatten()
    }

    fn get_focused(&self) -> Option<&CDO> {
        None
    }
}

impl InteractOpts for ACI {
    fn get_map<T>(&self, query: impl AsRef<str>, map: impl FnOnce(Value) -> T) -> Option<T> {
        self.data
            .options
            .iter()
            .find(|x| x.name.as_str() == query.as_ref())
            .and_then(|x| x.value.clone().map(map))
    }
}

impl InteractOpts for AI {
    fn get_map<T>(&self, query: impl AsRef<str>, map: impl FnOnce(Value) -> T) -> Option<T> {
        self.data
            .options
            .iter()
            .find(|x| x.name.as_str() == query.as_ref())
            .and_then(|x| x.value.clone().map(map))
    }

    fn get_focused(&self) -> Option<&CDO> {
        self.data.options.iter().find(|x| x.focused)
    }
}

impl InteractOpts for CDO {
    fn get_map<T>(&self, query: impl AsRef<str>, map: impl FnOnce(Value) -> T) -> Option<T> {
        self.options
            .iter()
            .find(|x| x.name.as_str() == query.as_ref())
            .and_then(|x| x.value.clone().map(map))
    }
}
