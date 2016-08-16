pub trait TryFrom<T>: Sized {
    type Err;
    fn try_from(T) -> Result<Self, Self::Err>;
}

pub trait TryInto<T>: Sized {
    type Err;
    fn try_into(self) -> Result<T, Self::Err>;
}

impl<T, U> TryInto<U> for T where U: TryFrom<T> {
    type Err = U::Err;

    fn try_into(self) -> Result<U, U::Err> {
        U::try_from(self)
    }
}
