//! This crate is pulled in from the [Firefly](https://github.com/GetFirefly/firefly) compiler, licensed under Apache 2.0

/// This trait represents anything that can be run as a pass.
///
/// Passes operate on an input value, and return either the same type, or a new type, depending on
/// the nature of the pass.
///
/// Implementations may represent a single pass, or an arbitrary number of passes that will be run
/// as a single unit.
///
/// Functions are valid implementations of `Pass` as long as their signature is `fn<I, O, E>(I) ->
/// Result<O, E>`.
pub trait Pass {
    type Input<'a>;
    type Output<'a>;
    type Error;

    /// Runs the pass on the given input
    ///
    /// Errors should be reported via the registered error handler,
    /// Passes should return `Err` to signal that the pass has failed
    /// and compilation should be aborted
    fn run<'a>(&mut self, input: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error>;

    /// Chains two passes together to form a new, fused pass
    fn chain<P, E>(self, pass: P) -> Chain<Self, P>
    where
        Self: Sized,
        E: From<Self::Error>,
        P: for<'a> Pass<Input<'a> = Self::Output<'a>, Error = E>,
    {
        Chain::new(self, pass)
    }
}
impl<P, T, U, E> Pass for &mut P
where
    P: for<'a> Pass<Input<'a> = T, Output<'a> = U, Error = E>,
{
    type Input<'a> = T;
    type Output<'a> = U;
    type Error = E;

    fn run<'a>(&mut self, input: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        (*self).run(input)
    }
}
impl<P, T, U, E> Pass for Box<P>
where
    P: ?Sized + for<'a> Pass<Input<'a> = T, Output<'a> = U, Error = E>,
{
    type Input<'a> = T;
    type Output<'a> = U;
    type Error = E;

    fn run<'a>(&mut self, input: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        (**self).run(input)
    }
}
impl<T, U, E> Pass for dyn FnMut(T) -> Result<U, E> {
    type Input<'a> = T;
    type Output<'a> = U;
    type Error = E;

    #[inline]
    fn run<'a>(&mut self, input: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        self(input)
    }
}

/// This struct is not meant to be used directly, but is instead produced
/// when chaining `Pass` implementations together. `Chain` itself implements `Pass`,
/// which is what enables us to chain together arbitrarily many passes into a single one.
pub struct Chain<A, B> {
    a: A,
    b: B,
}
impl<A, B> Chain<A, B> {
    fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}
impl<A, B> Clone for Chain<A, B>
where
    A: Clone,
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.a.clone(), self.b.clone())
    }
}
impl<A, B, AE, BE> Pass for Chain<A, B>
where
    A: for<'a> Pass<Error = AE>,
    B: for<'a> Pass<Input<'a> = <A as Pass>::Output<'a>, Error = BE>,
    BE: From<AE>,
{
    type Input<'a> = <A as Pass>::Input<'a>;
    type Output<'a> = <B as Pass>::Output<'a>;
    type Error = <B as Pass>::Error;

    fn run<'a>(&mut self, input: Self::Input<'a>) -> Result<Self::Output<'a>, Self::Error> {
        let u = self.a.run(input)?;
        self.b.run(u)
    }
}
