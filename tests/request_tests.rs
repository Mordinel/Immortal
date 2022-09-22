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

    use std::io;
    use std::io::ErrorKind;
    use std::str::Utf8Error;
    use immortal::immortal::*;
    use immortal::{Cookie, SameSite};

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
    fn test_request_cookies() {
        let mut buffer = b"".to_vec();
        buffer.append(&mut b"GET / HTTP/1.1\r\n".to_vec());
        buffer.append(&mut b"Host: 127.0.0.1\r\n".to_vec());
        buffer.append(&mut b"Cookie: id=9001; Secure; HttpOnly; SameSite=Lax; other_cookie=cookie_value; HttpOnly; SameSite=Strict; Domain=127.0.0.1; Path=/; last-cookie=short-lived; Max-Age=10;\r\n".to_vec());
        buffer.append(&mut b"Connection: close\r\n".to_vec());
        buffer.append(&mut b"\r\n".to_vec());
        let request = Request::new(buffer.as_mut_slice()).unwrap();

        let cookie_a = request.cookie("id").unwrap();
        let cookie_b = request.cookie("other_cookie").unwrap();
        let cookie_c = request.cookie("last-cookie").unwrap();

        assert_eq!(cookie_a.value, "9001");
        assert_eq!(cookie_a.secure, true);
        assert_eq!(cookie_a.http_only, true);
        assert_eq!(cookie_a.same_site, SameSite::Lax);

        assert_eq!(cookie_b.value, "cookie_value");
        assert_eq!(cookie_b.secure, false);
        assert_eq!(cookie_b.http_only, true);
        assert_eq!(cookie_b.same_site, SameSite::Strict);
        assert_eq!(cookie_b.domain, "127.0.0.1");
        assert_eq!(cookie_b.path, "/");

        assert_eq!(cookie_c.value, "short-lived");
        assert_eq!(cookie_c.secure, false);
        assert_eq!(cookie_c.http_only, false);
        assert_eq!(cookie_c.same_site, SameSite::Undefined);
        assert_eq!(cookie_c.max_age, 10i64);
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
    fn test_request_crlf() {
        let mut cases: Vec<Vec<u8>> = Vec::new();

        let mut buffer = b"GET / HTTP/1.1\r".to_vec();
        cases.push(buffer);

        buffer = b"GET / HTTP/1.1\r\n".to_vec();
        buffer.append(&mut b"Host: 127.0.0.1\r\n".to_vec());
        cases.push(buffer);

        for mut buf in cases {
            let _request = Request::new(buf.as_mut_slice());
        }
    }

    #[test]
    fn test_invalid_input() {
        let mut cases: Vec<Vec<u8>> = Vec::new();

        let mut buffer = b"".to_vec();
        cases.push(buffer);

        buffer = b"sajf;lkajd;fjkasdfkj;asdjf".to_vec();
        cases.push(buffer);

        buffer = b"GET /".to_vec();
        cases.push(buffer);

        buffer = b"GET / HTTPS/1.1".to_vec();
        cases.push(buffer);

        buffer = b"GET / HTTP/1.0".to_vec();
        cases.push(buffer);

        for mut buf in cases {
            let request = Request::new(buf.as_mut_slice());
            let error = request.unwrap_err();
            match error.downcast_ref::<io::Error>() {
                Some(err) => {
                    assert_eq!(err.kind(), ErrorKind::InvalidInput);
                }
                None => panic!("Expected to be io::Error"),
            }
        }
    }

    #[test]
    fn test_invalid_data() {
        let mut cases: Vec<Vec<u8>> = Vec::new();

        let mut buffer = b"GE\xffT / HTTP/1.1".to_vec();
        cases.push(buffer);

        buffer = b"GET /index\xff.html HTTP/1.1".to_vec();
        cases.push(buffer);

        buffer = b"GET /index.html?some_\xffparam=somevalue&a=b HTTP/1.1".to_vec();
        cases.push(buffer);

        buffer = b"GET / HT\xffTP/1.1".to_vec();
        cases.push(buffer);

        buffer = b"GET / HTTP/1\xff.1".to_vec();
        cases.push(buffer);

        buffer = b"GET / HTTP/1.1\r\n".to_vec();
        buffer.append(&mut b"X-Some-Valid-Header: valid header value\r\n".to_vec());
        buffer.append(&mut b"X-Some-Invalid-Header: in\xffvalid header value\r\n\r\n".to_vec());
        cases.push(buffer);

        for mut buf in cases {
            let request = Request::new(buf.as_mut_slice());
            let error = request.unwrap_err();
            assert!(match error.downcast_ref::<Utf8Error>() {
                Some(_) => true,
                None => false,
            });
        }
    }
}
