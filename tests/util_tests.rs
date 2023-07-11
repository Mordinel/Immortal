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
            Err(e) => assert_eq!(e.valid_up_to(), 0),
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
        assert_eq!(params.get("param_one").unwrap(), "val_one");
        assert_eq!(params.get("param_two").unwrap(), "val=two");
        assert_eq!(params.get("param_three").unwrap(), "val three");
    }

    #[test]
    fn test_parse_parameters_empty() {
        let param_string = String::from("");
        let params = parse_parameters(&param_string).unwrap();
        assert_eq!(params.len(), 0);
        assert_eq!(params.get("param_three"), None);
    }

    #[test]
    fn test_parse_headers() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"X-Some-Header: some value\r\n".to_vec());
        buffer.append(&mut b"X-Some-Other-Header: some other value".to_vec());
        let headers = parse_headers(buffer.as_mut_slice()).unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers.get("X-SOME-HEADER").unwrap(), "some value");
        assert_eq!(headers.get("X-SOME-OTHER-HEADER").unwrap(), "some other value");
    }

    #[test]
    fn test_parse_headers_empty() {
        let mut buffer = b"".to_vec();
        let headers = parse_headers(buffer.as_mut_slice()).unwrap();
        assert_eq!(headers.len(), 0);
        assert_eq!(headers.get("X-SOME-HEADER"), None);
    }

    #[test]
    fn test_parse_headers_junk() {
        let mut buffer = b"aksjdf;lkajsd;flkjas;dlfjk;".to_vec();
        let headers = parse_headers(buffer.as_mut_slice()).unwrap();
        assert_eq!(headers.len(), 0);
        assert_eq!(headers.get("X-SOME-HEADER"), None);
    }

    #[test]
    fn test_parse_headers_invalid() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"X-Some-Header: some value\r\n".to_vec());
        buffer.append(&mut b"X-Some-\xffOther-Header: some other value".to_vec());
        match parse_headers(buffer.as_mut_slice()) {
            Err(e) => assert_eq!(e.valid_up_to(), 34),
            _ => panic!("Expected Err()"),
        }
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
