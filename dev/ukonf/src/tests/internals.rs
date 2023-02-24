/*
 *        Copyright (c) 2023 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

mod unquote {
    use crate::ast::unquote;

    // Unit tests for unquote
    #[test]
    fn basic() {
        assert_eq!(unquote("test"), "test");
    }

    #[test]
    fn escape_sequence_n() {
        assert_eq!(unquote("test\\n"), "test\n");
    }

    #[test]
    fn escape_sequence_r() {
        assert_eq!(unquote("test\\r"), "test\r");
    }

    #[test]
    fn escape_sequence_t() {
        assert_eq!(unquote("test\\t"), "test\t");
    }

    #[test]
    fn escape_sequence_quote() {
        assert_eq!(unquote("test\\'"), "test\'");
    }

    #[test]
    fn escape_sequence_double_quote() {
        assert_eq!(unquote("test\\\""), "test\"");
    }

    #[test]
    fn escape_sequence_backslash() {
        assert_eq!(unquote("test\\\\"), "test\\");
    }

    #[test]
    fn escape_sequence_unicode() {
        assert_eq!(unquote("test\\u2010"), "test\u{2010}");
    }

    #[test]
    #[should_panic]
    fn unknown_escape_sequence() {
        unquote("test\\a");
    }

    #[test]
    #[should_panic]
    fn unexpected_end_of_string() {
        unquote("test\\");
    }
}
