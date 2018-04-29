/* Header */


pub fn is_zero<'a, T> (x: &'a T) -> bool
	where T: Default, for <'b> &'b T: PartialEq<&'b T>
{
	x == &T::default()
}
