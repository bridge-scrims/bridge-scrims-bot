#[macro_export]
macro_rules! id_impl {
    ($name:ty, $($what:ident),+) => {
        $(
            impl From<Vec<$what>> for $name {
                fn from(v: Vec<$what>) -> Self {
                    Self(v.into_iter().map(|x| x.0).collect())
                }
            }

            impl From<$name> for Vec<$what> {
                fn from(v: $name) -> Self {
                    v.0.into_iter().map(|x| $what(x)).collect()
                }
            }
        )+
    };
}
