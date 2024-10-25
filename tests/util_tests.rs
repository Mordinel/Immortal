#[cfg(test)]
mod tests {

    use immortal_http::util::*;

    #[test]
    fn test_url_decode_valid() {
        let to_decode = String::from("Hello%2C%20World%21");
        assert_eq!(url_decode(&to_decode).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_url_decode_empty() {
        let to_decode = String::from("");
        assert_eq!(url_decode(&to_decode).unwrap(), "");
    }

    #[test]
    fn test_url_decode_invalid_utf8() {
        let to_decode = String::from("%ff");
        match url_decode(&to_decode) {
            Err(ParseError::UrlDecodeNotUtf8(e)) => assert_eq!(e.valid_up_to(), 0),
            _ => panic!("Expected Err()"),
        }
    }

    #[test]
    fn test_url_decode_incomplete_encoding() {
        let to_decode_1 = String::from("%41%42%4");
        let to_decode_2 = String::from("%41%42%%");
        let to_decode_3 = String::from("%41%42%");
        assert_eq!(url_decode(&to_decode_1).unwrap(), "AB%4");
        assert_eq!(url_decode(&to_decode_2).unwrap(), "AB%%");
        assert_eq!(url_decode(&to_decode_3).unwrap(), "AB%");
    }

    #[test]
    fn test_parse_parameters() {
        let param_string = String::from("param_one=val_one&param_two=val=two&param_three=val%20three");
        let params = parse_parameters(&param_string).unwrap();
        assert_eq!(*params.iter().find(|(k,_)| *k == "param_one").map(|(_, v)| v).unwrap(), "val_one");
        assert_eq!(*params.iter().find(|(k,_)| *k == "param_two").map(|(_, v)| v).unwrap(), "val=two");
        assert_eq!(*params.iter().find(|(k,_)| *k == "param_three").map(|(_, v)| v).unwrap(), "val%20three");
    }

    #[test]
    fn test_parse_parameters_empty() {
        let param_string = String::from("");
        let params = parse_parameters(&param_string).unwrap();
        assert_eq!(params.len(), 0);
        assert_eq!(params.iter().find(|(k,_)| *k == "param_three").map(|(_,v)| v), None);
    }

    #[test]
    fn test_parse_headers() {
        assert_eq!(parse_header("X-Some-Header: some value"), Some(("X-Some-Header", "some value")));
        assert_eq!(parse_header("X-Some-Other-Header: some other value"), Some(("X-Some-Other-Header", "some other value")));
    }

    #[test]
    fn test_parse_headers_empty() {
        assert_eq!(parse_header("X-SOME-HEADER"), None);
    }

    #[test]
    fn test_parse_headers_junk() {
        assert_eq!(parse_header("aksjdf;lkajsd;flkjas;dlfjk;"), None);
    }

    #[test]
    fn test_param_name_validity_check() {
        assert_eq!(is_param_name_valid("Example_param_name"), true);
        assert_eq!(is_param_name_valid("Example-param-5"), true);
        assert_eq!(is_param_name_valid("Example param-5"), false);
        assert_eq!(is_param_name_valid("0-Example-Param"), false);
    }

    #[test]
    fn test_split_once() {
        let to_split = b"to split".to_vec();
        let (part_one, part_two) = split_once(to_split.as_slice(), b' ');
        assert_eq!(part_one, b"to");
        assert_eq!(part_two.unwrap(), b"split");
    }

    #[test]
    fn test_split_once_empty() {
        let to_split = b"".to_vec();
        let (part_one, part_two) = split_once(to_split.as_slice(), b' ');
        assert_eq!(part_one, b"");
        assert_eq!(part_two, None);
    }

    #[test]
    fn test_split_once_no_split() {
        let to_split = b"to split".to_vec();
        let (part_one, part_two) = split_once(to_split.as_slice(), b'x');
        assert_eq!(part_one, b"to split");
        assert_eq!(part_two, None);
    }

    #[test]
    fn test_split_once_no_second_part() {
        let to_split = b"to ".to_vec();
        let (part_one, part_two) = split_once(to_split.as_slice(), b' ');
        assert_eq!(part_one, b"to");
        assert_eq!(part_two, None);
    }
}
