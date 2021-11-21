#[macro_export]
macro_rules! dotenv {
    ($var:expr) => {{
        use ::dotenv::var;
        var($var).expect(format!("{} not specified", $var).as_str())
    }};
}
