macro_rules! newtype {
    ($name:ident, $t:ty) => { newtype!($name, $t, Default, Clone, Copy, PartialEq, Eq, Hash); };

    ($name:ident, $t:ty, $($derive:path),+) => {
        #[derive($($derive),+)]
        pub struct $name(pub $t);

        impl std::ops::Deref for $name {
            type Target = $t;

            fn deref(&self) -> &Self::Target { &self.0 }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }
    };
}
pub(crate) use newtype;
