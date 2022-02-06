use serde_json::{Map, Value};
use serenity::model::interactions::application_command::ApplicationCommandInteraction as ACI;
use serenity::model::interactions::application_command::ApplicationCommandInteractionDataOption as ACIDO;


/// Extracts options from an `ApplicationCommandInteraction`
/// Delegates [`serde_json::Value`] methods
pub trait InteractOpts: Sized {
    fn get_array(&self, query: impl AsRef<str>) -> Option<Vec<Value>>;
    fn get_bool(&self, query: impl AsRef<str>) -> Option<bool>;
    fn get_f64(&self, query: impl AsRef<str>) -> Option<f64>;
    fn get_i64(&self, query: impl AsRef<str>) -> Option<i64>;
    fn get_null(&self, query: impl AsRef<str>) -> Option<()>;
    fn get_object(&self, query: impl AsRef<str>) -> Option<Map<String, Value>>;
    fn get_str(&self, query: impl AsRef<str>) -> Option<String>;
    fn get_u64(&self, query: impl AsRef<str>) -> Option<u64>;
}

impl InteractOpts for ACI {
    fn get_array(&self, query: impl AsRef<str>) -> Option<Vec<Value>> {
        get_map(ResponseType::Aci(self.clone()), query, |x| {

            if let Value::Array(v) = x {
                Some(v)
            } else {
                None
            }
        })
        .flatten()
    }

    fn get_bool(&self, query: impl AsRef<str>) -> Option<bool> {
        get_map(ResponseType::Aci(self.clone()), query, |x| x.as_bool()).flatten()
    }

    fn get_f64(&self, query: impl AsRef<str>) -> Option<f64> {
        get_map(ResponseType::Aci(self.clone()), query, |x| x.as_f64()).flatten()
    }

    fn get_i64(&self, query: impl AsRef<str>) -> Option<i64> {
        get_map(ResponseType::Aci(self.clone()), query, |x| x.as_i64()).flatten()
    }

    fn get_null(&self, query: impl AsRef<str>) -> Option<()> {
        get_map(ResponseType::Aci(self.clone()), query, |x| x.as_null()).flatten()
    }

    fn get_object(&self, query: impl AsRef<str>) -> Option<Map<String, Value>> {
        get_map(ResponseType::Aci(self.clone()), query, |x| {
            x.as_object().cloned()
        })
        .flatten()
    }

    fn get_str(&self, query: impl AsRef<str>) -> Option<String> {
        get_map(ResponseType::Aci(self.clone()), query, |x| {

            if let Value::String(v) = x {
                Some(v)
            } else {
                None
            }
        })
        .flatten()
    }

    fn get_u64(&self, query: impl AsRef<str>) -> Option<u64> {
        get_map(ResponseType::Aci(self.clone()), query, |x| x.as_u64()).flatten()
    }
}

impl InteractOpts for ACIDO {
    fn get_array(&self, query: impl AsRef<str>) -> Option<Vec<Value>> {
        get_map(ResponseType::Acido(self.clone()), query, |x| {
            if let Value::Array(v) = x {
                Some(v)
            } else {
                None
            }
        })
        .flatten()
    }

    fn get_bool(&self, query: impl AsRef<str>) -> Option<bool> {
        get_map(ResponseType::Acido(self.clone()), query, |x| x.as_bool()).flatten()
    }

    fn get_f64(&self, query: impl AsRef<str>) -> Option<f64> {
        get_map(ResponseType::Acido(self.clone()), query, |x| x.as_f64()).flatten()
    }

    fn get_i64(&self, query: impl AsRef<str>) -> Option<i64> {
        get_map(ResponseType::Acido(self.clone()), query, |x| x.as_i64()).flatten()
    }

    fn get_null(&self, query: impl AsRef<str>) -> Option<()> {
        get_map(ResponseType::Acido(self.clone()), query, |x| x.as_null()).flatten()
    }

    fn get_object(&self, query: impl AsRef<str>) -> Option<Map<String, Value>> {
        get_map(ResponseType::Acido(self.clone()), query, |x| {
            x.as_object().cloned()
        })
        .flatten()
    }

    fn get_str(&self, query: impl AsRef<str>) -> Option<String> {
        get_map(ResponseType::Acido(self.clone()), query, |x| {
            if let Value::String(v) = x {
                Some(v)
            } else {
                None
            }
        })
        .flatten()
    }

    fn get_u64(&self, query: impl AsRef<str>) -> Option<u64> {
        get_map(ResponseType::Acido(self.clone()), query, |x| x.as_u64()).flatten()
    }
}

enum ResponseType {
    Aci(ACI),
    Acido(ACIDO),
}

fn get_map<T>(
    res: ResponseType,
    query: impl AsRef<str>,
    map: impl FnOnce(Value) -> T,
) -> Option<T> {
    match res {
        ResponseType::Aci(aci) => aci.data.options,
        ResponseType::Acido(acido) => acido.options,
    }
    .iter()
    .find(|x| x.name.as_str() == query.as_ref())
    .and_then(|x| x.value.clone().map(map))
}
