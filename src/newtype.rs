macro_rules! newtype {
    ($name:ident, $inner:ty) => {
        newtype!($name, $inner, Default, Clone, Copy, PartialEq, Eq, Hash);
    };

    ($name:ident, $inner:ty, $($derive:path),+) => {
        #[derive($($derive),+)]
        pub struct $name(pub $inner);

        impl std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}
pub(crate) use newtype;
