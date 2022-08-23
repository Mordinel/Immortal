/**
*     Copyright (C) 2022 Mason Soroka-Gill
*
*     This program is free software: you can redistribute it and/or modify
*     it under the terms of the GNU General Public License as published by
*     the Free Software Foundation, either version 3 of the License, or
*     (at your option) any later version.
*
*     This program is distributed in the hope that it will be useful,
*     but WITHOUT ANY WARRANTY; without even the implied warranty of
*     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*     GNU General Public License for more details.
*
*     You should have received a copy of the GNU General Public License
*     along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#[cfg(test)]
mod tests {

    use immortal::immortal::*;

    #[test]
    fn test_request() {
        let mut buffer = b"GET / HTTP/1.1".to_vec();
        let request = Request::new(buffer.as_mut_slice()).unwrap();

        assert_eq!(request.method, "GET");
        assert_eq!(request.document , "/");
        assert_eq!(request.protocol, "HTTP");
        assert_eq!(request.version, "1.1");
    }

    #[test]
    fn test_request_with_query() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"GET ".to_vec());
        buffer.append(&mut b"/?param_one=val_one&param_two=val=two&param_three=val%20three ".to_vec());
        buffer.append(&mut b"HTTP/1.1".to_vec());
        let request = Request::new(buffer.as_mut_slice()).unwrap();

        assert_eq!(request.get("param_one").unwrap(), "val_one");
        assert_eq!(request.get("param_two").unwrap(), "val=two");
        assert_eq!(request.get("param_three").unwrap(), "val three");
    }

    #[test]
    fn test_request_with_headers() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"POST / HTTP/1.1\r\n".to_vec());
        buffer.append(&mut b"Host: 127.0.0.1\r\n".to_vec());
        buffer.append(&mut b"Connection: keep-alive\r\n".to_vec());
        buffer.append(&mut b": bad header\r\n".to_vec());
        buffer.append(&mut b"8&&&x: bad header\r\n".to_vec());
        buffer.append(&mut b"X-Test-Header: test\r\n".to_vec());
        buffer.append(&mut b"Content-type: some_content_type\r\n".to_vec());
        buffer.append(&mut b"Content-Length: 13\r\n".to_vec());
        buffer.append(&mut b"\r\n".to_vec());
        buffer.append(&mut b"Hello, World!".to_vec());
        let request = Request::new(buffer.as_mut_slice()).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.host, "127.0.0.1");
        assert_eq!(request.connection, "keep-alive");
        assert!(request.keep_alive);
        assert_eq!(request.content_type, "some_content_type");
        assert_eq!(request.content_length, 13);
        assert_eq!(request.body, b"Hello, World!");
        assert_eq!(request.header(""), None);
        assert_eq!(request.header("8&&&x"), None);
        assert_eq!(request.header("X-Test-Header").unwrap(), "test");
    }

    #[test]
    fn test_request_post() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"POST / HTTP/1.1\r\n".to_vec());
        buffer.append(&mut b"Host: 127.0.0.1\r\n".to_vec());
        buffer.append(&mut b"Connection: close\r\n".to_vec());
        buffer.append(&mut b"Content-Type: application/x-www-form-urlencoded\r\n".to_vec());
        buffer.append(&mut b"\r\n".to_vec());
        buffer.append(&mut b"param_one=val_one&param_two=val=two&param_three=val%20three".to_vec());
        let request = Request::new(buffer.as_mut_slice()).unwrap();

        assert_eq!(request.post("param_one").unwrap(), "val_one");
        assert_eq!(request.post("param_two").unwrap(), "val=two");
        assert_eq!(request.post("param_three").unwrap(), "val three");
    }

    #[test]
    fn test_request_post_wrong_content_type() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"POST / HTTP/1.1\r\n".to_vec());
        buffer.append(&mut b"Host: 127.0.0.1\r\n".to_vec());
        buffer.append(&mut b"Connection: close\r\n".to_vec());
        buffer.append(&mut b"Content-Type: wrong content type\r\n".to_vec());
        buffer.append(&mut b"\r\n".to_vec());
        buffer.append(&mut b"param_one=val_one&param_two=val=two&param_three=val%20three".to_vec());
        let request = Request::new(buffer.as_mut_slice()).unwrap();

        assert_eq!(request.post("param_one"), None);
        assert_eq!(request.post("param_two"), None);
        assert_eq!(request.post("param_three"), None);
    }

    #[test]
    fn test_request_with_no_crlf() {
        let mut buffer = b"GET / HTTP/1.1\r".to_vec();
        let _request = Request::new(buffer.as_mut_slice());
    }

    #[test]
    fn test_request_with_no_double_crlf() {
        let mut buffer = b"GET / HTTP/1.1\r\n".to_vec();
        buffer.append(&mut b"Host: 127.0.0.1\r\n".to_vec());
        let _request = Request::new(buffer.as_mut_slice());
    }

    #[test]
    fn test_invalid_request_line_empty() {
        let mut buffer = b"".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid request line")));
    }

    #[test]
    fn test_invalid_request_line_junk() {
        let mut buffer = b"sajf;lkajd;fjkasdfkj;asdjf".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid request line")));
    }

    #[test]
    fn test_invalid_request_line_incomplete() {
        let mut buffer = b"GET /".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid request line")));
    }

    #[test]
    fn test_invalid_method_string() {
        let mut buffer = b"G\xffET / HTTP/1.1".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid method string: invalid utf-8 sequence of 1 bytes from index 1")));
    }

    #[test]
    fn test_invalid_document_string() {
        let mut buffer = b"GET /index\xff.html HTTP/1.1".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid document string: invalid utf-8 sequence of 1 bytes from index 6")));
    }

    #[test]
    fn test_invalid_query_string() {
        let mut buffer = b"GET /index.html?some_\xffparam=somevalue&a=b HTTP/1.1".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid query string: invalid utf-8 sequence of 1 bytes from index 5")));
    }

    #[test]
    fn test_invalid_proto_string_encoding() {
        let mut buffer = b"GET / HT\xffTP/1.1".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid protocol string: invalid utf-8 sequence of 1 bytes from index 2")));
    }

    #[test]
    fn test_invalid_proto_string_value() {
        let mut buffer = b"GET / HTTPS/1.1".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid protocol in proto string")));
    }

    #[test]
    fn test_invalid_version_string_encoding() {
        let mut buffer = b"GET / HTTP/1\xff.1".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid version string: invalid utf-8 sequence of 1 bytes from index 1")));
    }

    #[test]
    fn test_invalid_version_string_value() {
        let mut buffer = b"GET / HTTP/1.0".to_vec();
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid version in proto string")));
    }

    #[test]
    fn test_invalid_header_string() {
        let mut buffer = b"GET / HTTP/1.1\r\n".to_vec();
        buffer.append(&mut b"X-Some-Valid-Header: valid header value\r\n".to_vec());
        buffer.append(&mut b"X-Some-Invalid-Header: in\xffvalid header value\r\n\r\n".to_vec());
        let request = Request::new(buffer.as_mut_slice());
        assert_eq!(match request {
            Ok(_) => panic!("Expected to be invalid"),
            Err(e) => Err::<immortal::Request, String>(e),
        }, Err(String::from("Invalid header string: invalid utf-8 sequence of 1 bytes from index 66")));
    }
}
