/*
 * Not Sure what the name of this file/module should be. Helper seemed like the logical choice
 */

// Helper Trait and Impl for determining if a slice contains an element

// Use: used on any slice or array (probably vectors too)
/*
 * [1,2,3,4].does_contain(4);                            // => true
 * [1,2,3,4].does_contain(17);                           // => false
 * ["hello", "world", "foo", "bar"].does_contain("foo"); // => true
 *
 * let slc = &["hello", "world", "foo", "bar"];          type &[&str]
 * slc.does_contain("world");                            // => true        (searching for &str)
 * slc.does_contain("world".to_string());                // => also true   (searching for String)
 *
 * // Note: String, does not derive Copy, so a move will occur if you call does_contain() with
 * //       a String type argument. You can borrow this value by prepending an '&'
 * -----------------------------------------------------------------------------------------------
 * As long as two types are PartialEq ( T1: PartialEq<T2> ) then this _should_ work
 */

use std::cmp::PartialEq;

pub trait DoesContain<'a, T>{
    fn does_contain<V: PartialEq<T>>(&self, other: V) -> bool;
}

impl<'a, T> DoesContain<'a, T> for [T] {
    fn does_contain<V: PartialEq<T>>(&self, other: V) -> bool{
        ! self.iter().position(|x| other == *x ).is_none()
    }
}
