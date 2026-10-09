import unittest

from measure_wasm import rust_loc


class RustLocTests(unittest.TestCase):
    def test_comments_and_blank_lines_do_not_count(self):
        self.assertEqual(rust_loc('''
// Documentation
/* nested /* comment */ still comment */
fn main() { // This line contains code
    /* comment */ let url = "https://example.org";
}
'''), 3)

    def test_strings_do_not_hide_code_or_affect_test_module_braces(self):
        self.assertEqual(rust_loc('''
const TEXT: &str = r##"/* this is a string */"##;
#[cfg(test)]
mod tests {
    #[test]
    fn ignored() { assert_eq!("}", "{"); }
    fn character() { let x = '}'; }
}
fn production() {}
'''), 2)

    def test_multiple_test_modules_and_lifetimes(self):
        self.assertEqual(rust_loc('''
#[cfg(test)]
mod one { fn test() {} }
fn lifetime<'a>(input: &'a str) -> &'a str { input }
#[cfg(test)]
pub(crate) mod two { fn test() {} }
const VALUE: u8 = 1;
'''), 2)


if __name__ == "__main__":
    unittest.main()
