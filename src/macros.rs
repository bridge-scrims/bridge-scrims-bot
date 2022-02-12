#[macro_export]
macro_rules! dotenv {
    ($var:expr) => {{
        use ::dotenv::var;
        var($var).expect(format!("{} not specified", $var).as_str())
    }};
}

#[macro_export]
macro_rules! id_impl {
    ($name:ty, $($what:ident),+) => {
        $(
            impl From<Vec<$what>> for $name {
                fn from(v: Vec<$what>) -> Self {
                    Self(v.into_iter().map(|x| x.0).collect())
                }
            }

            impl Into<Vec<$what>> for $name {
                fn into(self) -> Vec<$what> {
                    self.0.into_iter().map(|x| $what(x)).collect()
                }
            }
        )+
    };
}
